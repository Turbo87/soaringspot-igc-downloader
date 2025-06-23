use clap::Parser;

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

fn normalize_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    if !url.starts_with("https://www.soaringspot.com/") {
        return Err("URL must be from www.soaringspot.com".into());
    }

    // Extract the path after the domain
    let domain_prefix = "https://www.soaringspot.com";
    let path = &url[domain_prefix.len()..];

    // Split the path and replace the language code
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 2 {
        return Err("Invalid URL format".into());
    }

    // Replace the language code (second part, first is empty due to leading /)
    let mut normalized_parts = parts;
    normalized_parts[1] = "en_gb";

    let normalized_path = normalized_parts.join("/");
    Ok(format!("{}{}", domain_prefix, normalized_path))
}
