mod date_utils;
mod parser;
mod url_utils;

use crate::url_utils::DailyUrlInfo;
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

    let client = reqwest::Client::new();
    let daily_urls = daily_urls_for_url(&client, &url).await?;

    let progress_bar = ProgressBar::new(daily_urls.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    let mut igc_files = vec![];
    for daily_url in daily_urls {
        let url = daily_url.to_daily_url();
        progress_bar.set_message(format!(
            "Loading results page for {} class on {}",
            daily_url.class, daily_url.date
        ));

        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            progress_bar.println(format!(
                "Failed to download HTML: HTTP {}",
                response.status()
            ));
            return Err(format!("HTTP error: {}", response.status()).into());
        }

        let html = response.text().await?;

        // Parse HTML and extract IGC file information
        let daily_igc_files = parse_igc_files(&html)?;
        progress_bar.println(format!(
            "✓ Processed: {} class on {}",
            daily_url.class, daily_url.date
        ));
        progress_bar.inc(1);

        igc_files.push((daily_url, daily_igc_files));
    }

    progress_bar.finish_with_message("Download complete!");

    if igc_files.is_empty() {
        println!("No IGC files found to download");
        return Ok(());
    }

    // Determine output directory and create directory structure
    let output_dir = args.output.unwrap_or_else(|| PathBuf::from("."));

    let total_files = igc_files
        .iter()
        .map(|(_, files)| files.len())
        .sum::<usize>();

    println!("Found {} IGC files", total_files);

    // Create progress bar
    let progress_bar = ProgressBar::new(total_files as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    for (daily_info, igc_files) in igc_files {
        // Create directory structure: {output}/{competition}/{class}/{date}/
        let date_str = daily_info.date.strftime("%Y-%m-%d").to_string();
        let target_dir = output_dir
            .join(&daily_info.competition)
            .join(&daily_info.class)
            .join(date_str);

        fs::create_dir_all(&target_dir).await?;
        println!("Downloading to: {}", target_dir.display());

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
    }

    progress_bar.finish_with_message("Download complete!");

    Ok(())
}

async fn daily_urls_for_url(
    client: &reqwest::Client,
    url: &Url,
) -> Result<Vec<DailyUrlInfo>, Box<dyn std::error::Error>> {
    let url_info = extract_url_info(url)?;
    Ok(match url_info {
        UrlInfo::Daily(daily) => vec![daily],
        UrlInfo::Class { competition, class } => {
            get_daily_urls_for_competition(client, &competition)
                .await?
                .into_iter()
                .filter(|info| info.class == class)
                .collect()
        }
        UrlInfo::Competition { competition } => {
            get_daily_urls_for_competition(client, &competition).await?
        }
    })
}

async fn get_daily_urls_for_competition(
    client: &reqwest::Client,
    competition: &str,
) -> Result<Vec<DailyUrlInfo>, Box<dyn std::error::Error>> {
    let url = format!("https://www.soaringspot.com/en_gb/{competition}/results");
    println!("Loading results page from: {}", url);

    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        return Err(format!("HTTP error {}: {}", response.status(), url).into());
    }

    let html = response.text().await?;
    parser::parse_daily_results(&html)
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
