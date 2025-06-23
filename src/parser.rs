use crate::url_utils::{DailyUrlInfo, UrlInfo, extract_url_info};
use html_escape::decode_html_entities;
use scraper::{Html, Selector};
use url::Url;

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

/// Extracts all daily result URLs from a competition results page.
///
/// Returns a list of [DailyUrlInfo] for each class and task.
pub fn parse_daily_results(html: &str) -> Result<Vec<DailyUrlInfo>, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html);
    let mut daily_results = Vec::new();

    // Select all links that point to daily results
    // Looking for: /en_gb/{competition}/results/{class}/task-{n}-on-{date}/daily
    let selector = Selector::parse(r#"a[href*="/results/"][href*="/daily"]"#)?;

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            // Construct full URL
            let full_url = format!("https://www.soaringspot.com{}", href);

            // Parse the URL to extract info
            if let Ok(url) = Url::parse(&full_url) {
                if let Ok(UrlInfo::Daily(daily_info)) = extract_url_info(&url) {
                    daily_results.push(daily_info);
                }
            }
        }
    }

    Ok(daily_results)
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

    #[test]
    fn test_parse_daily_results() {
        let html = include_str!("../tests/fixtures/results.html");
        let daily_results = parse_daily_results(html).unwrap();

        // Should find tasks for all three classes: club, standard, 15-meter
        // Each class should have 11 tasks (excluding practice tasks)
        assert_eq!(daily_results.len(), 33); // 3 classes × 11 tasks

        // Snapshot test for the structure
        insta::assert_debug_snapshot!(daily_results);
    }
}
