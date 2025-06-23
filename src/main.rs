#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025/results/club/task-10-on-2025-06-19/daily";

    println!("Downloading HTML from: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

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
