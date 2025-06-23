use html_escape::decode_html_entities;
use scraper::{Html, Selector};

#[derive(Debug, Clone)]
pub struct IgcFile {
    pub callsign: String,
    pub download_url: String,
}

pub fn parse_igc_files(html: &str) -> Result<Vec<IgcFile>, Box<dyn std::error::Error>> {
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
    // The data_content contains HTML-encoded content
    // We need to decode it and extract the download URL with ?dl=1

    // Decode HTML entities using the html-escape library
    let decoded = decode_html_entities(data_content);

    // Parse the decoded HTML fragment using scraper
    let fragment = Html::parse_fragment(&decoded);

    // Select all links that contain "download-contest-flight" and have ?dl=1
    let link_selector = Selector::parse(r#"a[href*="download-contest-flight"][href*="dl=1"]"#)
        .expect("Invalid CSS selector");

    // Find the first matching link and extract its href
    fragment
        .select(&link_selector)
        .next()
        .and_then(|element| element.value().attr("href"))
        .map(|href| format!("https://www.soaringspot.com{}", href))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_igc_files_snapshot() {
        let html = include_str!("../tests/fixtures/day.html");
        let igc_files = parse_igc_files(html).expect("Failed to parse IGC files");

        insta::assert_debug_snapshot!(igc_files);
    }
}
