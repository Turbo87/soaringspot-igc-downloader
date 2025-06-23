mod date_utils;
mod parser;
mod url_utils;

use clap::Parser;
use date_utils::date_to_igc_filename_prefix;
use indicatif::{ProgressBar, ProgressStyle};
use parser::parse_igc_files;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use url::Url;
use url_utils::{UrlInfo, extract_url_info, normalize_url_inplace};

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
    let daily_info = match &url_info {
        UrlInfo::Daily(daily_info) => {
            println!(
                "Competition: {}, Class: {}, Date: {}, Task: {}",
                daily_info.competition, daily_info.class, daily_info.date, daily_info.task_number
            );
            daily_info.clone()
        }
        UrlInfo::Class { competition, class } => {
            println!("Competition: {}, Class: {} (all dates)", competition, class);
            // TODO: Implement discovery of all dates for this class
            return Err("Multi-date downloads not yet implemented".into());
        }
        UrlInfo::Competition { competition } => {
            println!("Competition: {} (all classes and dates)", competition);
            // TODO: Implement discovery of all classes and dates
            return Err("Multi-class downloads not yet implemented".into());
        }
    };

    println!("Downloading HTML from: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        let html = response.text().await?;
        println!("Successfully downloaded HTML ({} bytes)", html.len());

        // Parse HTML and extract IGC file information
        let igc_files = parse_igc_files(&html)?;
        println!("Found {} IGC files", igc_files.len());

        if igc_files.is_empty() {
            println!("No IGC files found to download");
            return Ok(());
        }

        // Determine output directory and create directory structure
        let output_dir = args.output.unwrap_or_else(|| PathBuf::from("."));

        // Create directory structure: {output}/{competition}/{class}/{date}/
        let date_str = daily_info.date.strftime("%Y-%m-%d").to_string();
        let target_dir = output_dir
            .join(&daily_info.competition)
            .join(&daily_info.class)
            .join(date_str);

        fs::create_dir_all(&target_dir).await?;
        println!("Downloading to: {}", target_dir.display());

        // Create progress bar
        let progress_bar = ProgressBar::new(igc_files.len() as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-")
        );

        // Generate date prefix for filenames
        let date_prefix = date_to_igc_filename_prefix(daily_info.date);

        // Download each IGC file
        for igc_file in igc_files {
            let filename = format!("{}_{}.igc", date_prefix, igc_file.callsign);
            let file_path = target_dir.join(&filename);

            progress_bar.set_message(format!("Downloading {}", filename));

            // Skip if file already exists
            if file_path.exists() {
                progress_bar.println(format!("⏭ Skipping existing file: {}", filename));
                progress_bar.inc(1);
                continue;
            }

            // Download to temporary file first
            match download_igc_file(&client, &igc_file.download_url, &file_path).await {
                Ok(_) => {
                    progress_bar.println(format!("✓ Downloaded: {}", filename));
                }
                Err(e) => {
                    progress_bar.println(format!("✗ Failed to download {}: {}", filename, e));
                }
            }

            progress_bar.inc(1);
        }

        progress_bar.finish_with_message("Download complete!");
    } else {
        eprintln!("Failed to download HTML: HTTP {}", response.status());
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    Ok(())
}

async fn download_igc_file(
    client: &reqwest::Client,
    url: &str,
    final_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(format!("HTTP error {}: {}", response.status(), url).into());
    }

    let content = response.bytes().await?;

    // Create a temporary file
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path();

    // Write content to temporary file
    let mut file = fs::File::create(temp_path).await?;
    file.write_all(&content).await?;
    file.shutdown().await?;
    drop(file);

    // Atomically move temp file to final location
    fs::rename(temp_path, final_path).await?;

    Ok(())
}
