use jiff::civil::Date;
use url::Url;

#[derive(Debug)]
pub struct UrlInfo {
    pub class: String,
    pub date: Date,
}

pub fn normalize_url_inplace(url: &mut Url) -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

pub fn extract_url_info(url: &Url) -> Result<UrlInfo, Box<dyn std::error::Error>> {
    let segments: Vec<&str> = url.path_segments().ok_or("Invalid URL path")?.collect();

    // Expected pattern: /en_gb/{event}/results/{class}/task-{n}-on-{date}/daily
    if segments.len() < 6 {
        return Err("URL does not contain enough path segments for daily results".into());
    }

    // Verify "results" is the third segment
    if segments[2] != "results" {
        return Err("'results' must be the third path segment".into());
    }

    // Verify the URL ends with "daily"
    if segments[5] != "daily" {
        return Err("URL must end with '/daily' for daily results".into());
    }

    // The class is the fourth segment (index 3)
    let class = segments[3].to_string();

    // The task segment should be the fifth segment (index 4)
    let task_segment = segments[4];

    // Verify it has the expected task-{n}-on-{date} pattern
    if !task_segment.starts_with("task-") || !task_segment.contains("-on-") {
        return Err("Fifth segment must be a task segment with date (task-{n}-on-{date})".into());
    }

    // Extract date from task-{n}-on-{date}
    let (_, date_str) = task_segment
        .split_once("-on-")
        .ok_or("Could not extract date from task segment")?;

    // Parse the date string using jiff
    let date = Date::strptime("%Y-%m-%d", date_str)
        .map_err(|e| format!("Failed to parse date '{}': {}", date_str, e))?;

    Ok(UrlInfo { class, date })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url_inplace() {
        // Test HTTP to HTTPS conversion
        let mut url = Url::parse("http://www.soaringspot.com/en_gb/test").unwrap();
        normalize_url_inplace(&mut url).unwrap();
        insta::assert_snapshot!(url, @"https://www.soaringspot.com/en_gb/test");

        // Test adding www prefix
        let mut url = Url::parse("https://soaringspot.com/en_gb/test").unwrap();
        normalize_url_inplace(&mut url).unwrap();
        insta::assert_snapshot!(url, @"https://www.soaringspot.com/en_gb/test");

        // Test language code normalization
        let mut url = Url::parse("https://www.soaringspot.com/de/test").unwrap();
        normalize_url_inplace(&mut url).unwrap();
        insta::assert_snapshot!(url, @"https://www.soaringspot.com/en_gb/test");

        let mut url = Url::parse("https://www.soaringspot.com/fr_fr/test").unwrap();
        normalize_url_inplace(&mut url).unwrap();
        insta::assert_snapshot!(url, @"https://www.soaringspot.com/en_gb/test");

        // Test invalid scheme error
        let mut url = Url::parse("ftp://www.soaringspot.com/en_gb/test").unwrap();
        let result = normalize_url_inplace(&mut url);
        insta::assert_snapshot!(result.unwrap_err(), @"URL must use HTTP or HTTPS scheme");

        // Test invalid host error
        let mut url = Url::parse("https://invalid.com/en_gb/test").unwrap();
        let result = normalize_url_inplace(&mut url);
        insta::assert_snapshot!(result.unwrap_err(), @"URL must be from soaringspot.com or www.soaringspot.com");
    }

    #[test]
    fn test_extract_url_info() {
        // Test valid daily results URL
        let url = "https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025/results/club/task-10-on-2025-06-19/daily";
        let url = Url::parse(url).unwrap();
        let info = extract_url_info(&url).unwrap();
        insta::assert_debug_snapshot!(info, @r#"
        UrlInfo {
            class: "club",
            date: 2025-06-19,
        }
        "#);

        // Test different class
        let url = "https://www.soaringspot.com/en_gb/competition/results/standard/task-5-on-2024-07-15/daily";
        let url = Url::parse(url).unwrap();
        let info = extract_url_info(&url).unwrap();
        insta::assert_debug_snapshot!(info, @r#"
        UrlInfo {
            class: "standard",
            date: 2024-07-15,
        }
        "#);

        // Test error cases
        let url = "https://www.soaringspot.com/en_gb/test";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"URL does not contain enough path segments for daily results");

        let url = "https://www.soaringspot.com/en_gb/test/invalid/club/task-1-on-2025-01-01/daily";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"'results' must be the third path segment");

        let url = "https://www.soaringspot.com/en_gb/test/results/club/task-1-on-2025-01-01/other";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"URL must end with '/daily' for daily results");

        let url = "https://www.soaringspot.com/en_gb/test/results/club/invalid-format/daily";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"Fifth segment must be a task segment with date (task-{n}-on-{date})");

        let url =
            "https://www.soaringspot.com/en_gb/test/results/club/task-1-on-invalid-date/daily";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"Failed to parse date 'invalid-date': strptime parsing failed: %Y failed: failed to parse year: invalid number, no digits found");
    }
}
