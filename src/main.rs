mod parser;

use clap::Parser;
use parser::parse_igc_files;
use std::path::PathBuf;
use url::Url;

#[derive(Debug)]
struct UrlInfo {
    class: String,
    date: String,
}

#[derive(Parser)]
#[command(name = "soaringspot-igc-downloader")]
#[command(about = "Downloads IGC files from SoaringSpot competition results")]
#[command(version)]
struct Args {
    /// SoaringSpot URL to download from
    url: String,

    /// Output directory for IGC files (defaults to current directory)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Parse and normalize URL, then extract info
    let mut url = Url::parse(&args.url)?;
    normalize_url_inplace(&mut url)?;
    let url_info = extract_url_info(&url)?;
    println!("Class: {}, Date: {}", url_info.class, url_info.date);

    println!("Downloading HTML from: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        let html = response.text().await?;
        println!("Successfully downloaded HTML ({} bytes)", html.len());

        // Parse HTML and extract IGC file information
        let igc_files = parse_igc_files(&html)?;
        println!("Found {} IGC files:", igc_files.len());

        // Determine output directory
        let output_dir = args.output.unwrap_or_else(|| PathBuf::from("."));
        println!("Output directory: {}", output_dir.display());

        for igc_file in &igc_files {
            println!("  {} -> {}", igc_file.callsign, igc_file.download_url);
        }
    } else {
        eprintln!("Failed to download HTML: HTTP {}", response.status());
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    Ok(())
}

fn normalize_url_inplace(url: &mut Url) -> Result<(), Box<dyn std::error::Error>> {
    // Normalize scheme to HTTPS
    if url.scheme() == "http" {
        url.set_scheme("https")
            .map_err(|_| "Failed to convert to HTTPS")?;
    } else if url.scheme() != "https" {
        return Err("URL must use HTTP or HTTPS scheme".into());
    }

    // Validate and normalize the host
    let host = url.host_str().ok_or("Invalid URL - missing host")?;
    if host == "soaringspot.com" {
        // Add www prefix if missing
        url.set_host(Some("www.soaringspot.com"))?;
    } else if host != "www.soaringspot.com" {
        return Err("URL must be from soaringspot.com or www.soaringspot.com".into());
    }

    // Parse the path segments
    let mut segments: Vec<&str> = url.path_segments().ok_or("Invalid URL path")?.collect();

    if segments.is_empty() {
        return Err("Invalid URL format - missing path segments".into());
    }

    // Replace the language code (first segment) with en_gb
    segments[0] = "en_gb";

    // Reconstruct the path
    let new_path = format!("/{}", segments.join("/"));
    url.set_path(&new_path);

    Ok(())
}

fn extract_url_info(url: &Url) -> Result<UrlInfo, Box<dyn std::error::Error>> {
    let segments: Vec<&str> = url.path_segments().ok_or("Invalid URL path")?.collect();
    // Expected pattern: /en_gb/{event}/results/{class}/task-{n}-on-{date}/daily
    if segments.len() < 6 {
        return Err("URL does not contain enough path segments for daily results".into());
    }

    // Verify "results" is the third segment
    if segments[2] != "results" {
        return Err("'results' must be the third path segment".into());
    }

    // Verify the URL ends with "daily"
    if segments[5] != "daily" {
        return Err("URL must end with '/daily' for daily results".into());
    }

    // The class is the fourth segment (index 3)
    let class = segments[3].to_string();

    // The task segment should be the fifth segment (index 4)
    let task_segment = segments[4];

    // Verify it has the expected task-{n}-on-{date} pattern
    if !task_segment.starts_with("task-") || !task_segment.contains("-on-") {
        return Err("Fifth segment must be a task segment with date (task-{n}-on-{date})".into());
    }

    // Extract date from task-{n}-on-{date}
    let (_, date) = task_segment
        .split_once("-on-")
        .ok_or("Could not extract date from task segment")?;

    Ok(UrlInfo {
        class,
        date: date.to_string(),
    })
}
