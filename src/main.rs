use clap::Parser;
use scraper::{Html, Selector};
use url::Url;

#[derive(Debug, Clone)]
struct IgcFile {
    callsign: String,
    download_url: String,
}

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

        // Parse HTML and extract IGC file information
        let igc_files = parse_igc_files(&html)?;
        println!("Found {} IGC files:", igc_files.len());

        for igc_file in &igc_files {
            println!("  {} -> {}", igc_file.callsign, igc_file.download_url);
        }
    } else {
        eprintln!("Failed to download HTML: HTTP {}", response.status());
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    Ok(())
}

fn normalize_url(url_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut url = Url::parse(url_str)?;

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

    Ok(url.to_string())
}

fn parse_igc_files(html: &str) -> Result<Vec<IgcFile>, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html);
    let mut igc_files = Vec::new();

    // Select all elements with data-content attributes containing download links
    let selector = Selector::parse(r#"[data-content*="download-contest-flight"]"#)?;

    for element in document.select(&selector) {
        // Extract the data-content attribute
        if let Some(data_content) = element.value().attr("data-content") {
            // Decode HTML entities and extract the download URL
            if let Some(download_url) = extract_download_url(data_content) {
                // Get the callsign from the text content of this cell
                let callsign = element.text().collect::<String>().trim().to_string();

                if !callsign.is_empty() {
                    igc_files.push(IgcFile {
                        callsign,
                        download_url,
                    });
                }
            }
        }
    }

    Ok(igc_files)
}

fn extract_download_url(data_content: &str) -> Option<String> {
    // The data_content contains HTML-encoded content like:
    // "&#x2F;en_gb&#x2F;download-contest-flight&#x2F;5039-10179576293&#x3F;dl&#x3D;1"
    // We need to decode it and extract the URL

    // Simple HTML entity decoder for the specific entities we need
    let decoded = data_content
        .replace("&#x2F;", "/")
        .replace("&#x3D;", "=")
        .replace("&#x3B;", ";")
        .replace("&#x20;", " ")
        .replace("&#x0A;", "\n");

    // Look for the download URL with ?dl=1 parameter (second occurrence)
    let mut last_url = None;
    let mut search_start = 0;

    while let Some(start_pos) =
        decoded[search_start..].find(r#"href="/en_gb/download-contest-flight/"#)
    {
        let actual_pos = search_start + start_pos;
        let href_start = actual_pos + 6; // Skip 'href="'

        if let Some(end_pos) = decoded[href_start..].find('"') {
            let url_part = &decoded[href_start..href_start + end_pos];
            last_url = Some(format!("https://www.soaringspot.com{}", url_part));
            search_start = href_start + end_pos + 1;
        } else {
            break;
        }
    }

    last_url
}
