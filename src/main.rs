use clap::Parser;
use url::Url;

#[derive(Parser)]
#[command(name = "soaringspot-igc-downloader")]
#[command(about = "Downloads IGC files from SoaringSpot competition results")]
#[command(version)]
struct Args {
    /// SoaringSpot URL to download from
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Normalize URL to use en_gb language code
    let url = normalize_url(&args.url)?;
    println!("Downloading HTML from: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    if response.status().is_success() {
        let html = response.text().await?;
        println!("Successfully downloaded HTML ({} bytes)", html.len());

        // For now, just print a snippet to verify it worked
        if html.len() > 200 {
            println!("HTML preview: {}...", &html[..200]);
        } else {
            println!("HTML content: {}", html);
        }
    } else {
        eprintln!("Failed to download HTML: HTTP {}", response.status());
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    Ok(())
}

fn normalize_url(url_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut url = Url::parse(url_str)?;

    // Validate that it's a SoaringSpot URL
    if url.host_str() != Some("www.soaringspot.com") {
        return Err("URL must be from www.soaringspot.com".into());
    }

    if url.scheme() != "https" {
        return Err("URL must use HTTPS".into());
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

    Ok(url.to_string())
}
