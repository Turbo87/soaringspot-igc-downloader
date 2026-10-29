mod cup;
mod date_utils;
mod parser;
mod results_csv;
mod url_utils;

use crate::url_utils::DailyUrlInfo;
use clap::Parser;
use date_utils::date_to_igc_filename_prefix;
use indicatif::{ProgressBar, ProgressStyle};
use parser::{extract_task_json, parse_igc_files};
use results_csv::results_csv;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tempfile::NamedTempFile;
use tokio::fs;
use tokio::time::{Duration, sleep};
use url::Url;
use url_utils::{UrlInfo, extract_url_info};

#[derive(Parser)]
#[command(about, version)]
struct Args {
    /// SoaringSpot URL to download from
    ///
    /// Examples:
    ///
    /// - https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025
    ///
    /// - https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025/results/standard
    ///
    /// - https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025/results/club/task-4-on-2025-06-12/daily
    url: Url,

    /// Output directory for IGC files (defaults to current directory)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Include practice days in the downloads
    #[arg(long)]
    include_practice: bool,
}

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let args = Args::parse();
    let output_dir = args.output.unwrap_or_else(|| PathBuf::from("."));

    let client = reqwest::Client::new();
    let daily_urls = daily_urls_for_url(&client, &args.url, args.include_practice).await?;

    let progress_bar = ProgressBar::new(daily_urls.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    let mut igc_files = vec![];
    let mut failures = vec![];
    for daily_url in daily_urls {
        let url = daily_url.to_daily_url();
        progress_bar.set_message(format!(
            "Loading results page for {} class on {}",
            daily_url.class, daily_url.date
        ));

        let html = String::from_utf8(fetch_with_retry(&client, &url, &progress_bar).await?)?;

        // Parse HTML and extract IGC file information
        let daily_igc_files = parse_igc_files(&html)?;

        let target_dir = day_directory(&output_dir, &daily_url);
        if let Err(error) =
            save_task_and_results(&client, &daily_url, &html, &target_dir, &progress_bar).await
        {
            let failure = format!("{} class on {}: {}", daily_url.class, daily_url.date, error);
            progress_bar.println(format!("✗ Failed to save task and results for {}", failure));
            failures.push(failure);
        }

        progress_bar.println(format!(
            "✓ Processed: {} class on {}",
            daily_url.class, daily_url.date
        ));
        progress_bar.inc(1);

        if !daily_igc_files.is_empty() {
            igc_files.push((daily_url, daily_igc_files));
        }
    }

    progress_bar.finish_with_message("Download complete!");

    if igc_files.is_empty() {
        println!("No IGC files found to download");
        return Ok(report_failures(&failures));
    }

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
        let target_dir = day_directory(&output_dir, &daily_info);
        fs::create_dir_all(&target_dir).await?;
        progress_bar.println(format!("Downloading to: {}", target_dir.display()));

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
            match download_igc_file(&client, &igc_file.download_url, &file_path, &progress_bar)
                .await
            {
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

    Ok(report_failures(&failures))
}

/// Returns `{output}/{competition}/{class}/{date}/`.
fn day_directory(output_dir: &Path, daily_info: &DailyUrlInfo) -> PathBuf {
    output_dir
        .join(&daily_info.competition)
        .join(&daily_info.class)
        .join(daily_info.date.to_string())
}

fn report_failures(failures: &[String]) -> ExitCode {
    if failures.is_empty() {
        return ExitCode::SUCCESS;
    }

    let days = failures.len();
    eprintln!("Failed to save task and results for {days} day(s):");
    for failure in failures {
        eprintln!("  {}", failure);
    }
    ExitCode::FAILURE
}

/// Fetches the task page and saves it together with the results page, the
/// results as CSV file, the extracted task data, and the task as CUP file.
/// Nothing is written unless all of them are available, so existing files
/// are kept on failure.
async fn save_task_and_results(
    client: &reqwest::Client,
    daily_info: &DailyUrlInfo,
    results_html: &str,
    target_dir: &Path,
    progress_bar: &ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let task_page = fetch_with_retry(client, &daily_info.to_task_url(), progress_bar).await?;
    let task_html = String::from_utf8(task_page)?;
    let task_json = extract_task_json(&task_html)?.to_owned();
    let (task_cup, warnings) = cup::task_cup(&serde_json::from_str(&task_json)?)?;
    let day = format!("{} class on {}", daily_info.class, daily_info.date);
    for warning in warnings {
        progress_bar.println(format!("⚠ {day}: {warning}"));
    }

    let mut files = vec![
        ("results.html", results_html.as_bytes().to_vec()),
        ("task.html", task_html.into_bytes()),
        ("task.json", task_json.into_bytes()),
        ("task.cup", task_cup.into_bytes()),
    ];
    if let Some(results_csv) = results_csv(results_html)? {
        files.push(("results.csv", results_csv.into_bytes()));
    }

    let target_dir = target_dir.to_owned();
    tokio::task::spawn_blocking(move || write_day_files(&target_dir, &files)).await??;

    Ok(())
}

fn write_day_files(dir: &Path, files: &[(&str, Vec<u8>)]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (name, content) in files {
        write_atomically(&dir.join(name), content)?;
    }
    Ok(())
}

async fn daily_urls_for_url(
    client: &reqwest::Client,
    url: &Url,
    include_practice: bool,
) -> Result<Vec<DailyUrlInfo>, Box<dyn std::error::Error>> {
    let url_info = extract_url_info(url)?;
    Ok(match url_info {
        UrlInfo::Daily(daily) => vec![daily],
        UrlInfo::Class { competition, class } => {
            get_daily_urls_for_competition(client, &competition, include_practice)
                .await?
                .into_iter()
                .filter(|info| info.class == class)
                .collect()
        }
        UrlInfo::Competition { competition } => {
            get_daily_urls_for_competition(client, &competition, include_practice).await?
        }
    })
}

async fn get_daily_urls_for_competition(
    client: &reqwest::Client,
    competition: &str,
    include_practice: bool,
) -> Result<Vec<DailyUrlInfo>, Box<dyn std::error::Error>> {
    let url = format!("https://www.soaringspot.com/en_gb/{competition}/results");
    println!("Loading results page from: {}", url);

    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        return Err(format!("HTTP error {}: {}", response.status(), url).into());
    }

    let html = response.text().await?;
    Ok(parser::parse_daily_results(&html)?
        .into_iter()
        .filter(|info| {
            // Filter out practice days if not requested
            include_practice || !info.is_practice_day()
        })
        .collect())
}

async fn download_igc_file(
    client: &reqwest::Client,
    url: &str,
    final_path: &Path,
    progress_bar: &ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fetch_with_retry(client, url, progress_bar).await?;

    let final_path = final_path.to_path_buf();
    tokio::task::spawn_blocking(move || write_atomically(&final_path, &content)).await??;

    Ok(())
}

/// Fetches `url`, retrying with exponential backoff while SoaringSpot
/// responds with its "Too Many Requests" page.
async fn fetch_with_retry(
    client: &reqwest::Client,
    url: &str,
    progress_bar: &ProgressBar,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Retry parameters
    const MAX_RETRIES: u32 = 5;
    const INITIAL_DELAY_MS: u64 = 1000; // 1 second
    const BACKOFF_FACTOR: u64 = 2; // Exponential backoff multiplier

    let mut retry_count = 0;
    let mut delay_ms = INITIAL_DELAY_MS;

    loop {
        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(format!("HTTP error {}: {}", response.status(), url).into());
        }

        let content = response.bytes().await?;

        // check for "Too Many Requests"
        if std::str::from_utf8(&content)
            .unwrap_or("")
            .contains("<h1>Too Many Requests</h1>")
        {
            if retry_count >= MAX_RETRIES {
                return Err(format!(
                    "Exceeded maximum retries ({}) for Too Many Requests: {}",
                    MAX_RETRIES, url
                )
                .into());
            }

            retry_count += 1;
            progress_bar.println(format!(
                "Received 'Too Many Requests' response in content. Retrying in {} ms (attempt {}/{})",
                delay_ms, retry_count, MAX_RETRIES
            ));

            sleep(Duration::from_millis(delay_ms)).await;
            delay_ms *= BACKOFF_FACTOR; // Exponential backoff
            continue;
        }

        return Ok(content.into());
    }
}

/// Writes `content` to `path` via a temporary file in the same directory,
/// so that the final rename never crosses a filesystem boundary.
fn write_atomically(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut temp_file = NamedTempFile::new_in(dir)?;
    temp_file.write_all(content)?;
    temp_file.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomically_replaces_file_without_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("56J_AM.igc");

        write_atomically(&path, b"first").unwrap();
        write_atomically(&path, b"second").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        let entries = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(entries, 1);
    }

    #[test]
    fn write_day_files_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target_dir = dir.path().join("competition/club/2025-06-19");
        let files = vec![
            ("task.json", b"{}".to_vec()),
            ("results.html", b"<html>".to_vec()),
        ];

        write_day_files(&target_dir, &files).unwrap();

        for (name, content) in &files {
            assert_eq!(&std::fs::read(target_dir.join(name)).unwrap(), content);
        }
    }
}
