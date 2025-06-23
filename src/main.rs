mod date_utils;
mod parser;
mod url_utils;

use clap::Parser;
use parser::parse_igc_files;
use std::path::PathBuf;
use url::Url;
use url_utils::{extract_url_info, normalize_url_inplace};

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
