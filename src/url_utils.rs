use jiff::civil::Date;
use url::Url;

#[derive(Debug, Clone)]
pub struct DailyUrlInfo {
    pub competition: String,
    pub class: String,
    pub date: Date,
}

#[derive(Debug)]
pub enum UrlInfo {
    /// Daily results - has competition, class, and date
    Daily(DailyUrlInfo),
    /// Class results - has competition and class, needs to discover all dates
    Class { competition: String, class: String },
    /// All competition results - has competition only, needs to discover all classes and dates
    Competition { competition: String },
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
    let mut segments = url.path_segments().ok_or("Invalid URL path")?;

    // Get language code
    let _language = segments
        .next()
        .ok_or("Invalid URL format - missing path segments")?;

    // Get competition name
    let competition = segments
        .next()
        .ok_or("URL must contain competition name")?
        .to_string();

    // Pattern: /en_gb/{competition}
    let Some(third_segment) = segments.next() else {
        return Ok(UrlInfo::Competition { competition });
    };

    // Must be a results URL from here
    if third_segment != "results" {
        return Err("Unsupported URL format".into());
    }

    // Pattern: /en_gb/{competition}/results
    let Some(class) = segments.next() else {
        return Ok(UrlInfo::Competition { competition });
    };

    let class = class.to_string();

    // Pattern: /en_gb/{competition}/results/{class}
    let Some(task) = segments.next() else {
        return Ok(UrlInfo::Class { competition, class });
    };

    // Pattern: /en_gb/{competition}/results/{class}/task-N-on-DATE(/daily)?
    if !task.starts_with("task-") || !task.contains("-on-") {
        return Err("Unsupported URL format".into());
    }

    // Extract date from task-{n}-on-{date}
    let (_, date_str) = task
        .split_once("-on-")
        .ok_or("Could not extract date from task segment")?;

    // Parse the date string using jiff
    let date = Date::strptime("%Y-%m-%d", date_str)
        .map_err(|e| format!("Failed to parse date '{}': {}", date_str, e))?;

    Ok(UrlInfo::Daily(DailyUrlInfo {
        competition,
        class,
        date,
    }))
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
        Daily(
            DailyUrlInfo {
                competition: "39th-fai-world-gliding-championships-tabor-2025",
                class: "club",
                date: 2025-06-19,
            },
        )
        "#);

        // Test different class
        let url = "https://www.soaringspot.com/en_gb/competition/results/standard/task-5-on-2024-07-15/daily";
        let url = Url::parse(url).unwrap();
        let info = extract_url_info(&url).unwrap();
        insta::assert_debug_snapshot!(info, @r#"
        Daily(
            DailyUrlInfo {
                competition: "competition",
                class: "standard",
                date: 2024-07-15,
            },
        )
        "#);

        // Test competition URL
        let url = "https://www.soaringspot.com/en_gb/test";
        let url = Url::parse(url).unwrap();
        let info = extract_url_info(&url).unwrap();
        insta::assert_debug_snapshot!(info, @r#"
        Competition {
            competition: "test",
        }
        "#);

        // Test class URL
        let url = "https://www.soaringspot.com/en_gb/test/results/club";
        let url = Url::parse(url).unwrap();
        let info = extract_url_info(&url).unwrap();
        insta::assert_debug_snapshot!(info, @r#"
        Class {
            competition: "test",
            class: "club",
        }
        "#);

        // Test all competition results URL
        let url = "https://www.soaringspot.com/en_gb/test/results";
        let url = Url::parse(url).unwrap();
        let info = extract_url_info(&url).unwrap();
        insta::assert_debug_snapshot!(info, @r#"
        Competition {
            competition: "test",
        }
        "#);

        // Test error cases
        let url = "https://www.soaringspot.com/en_gb";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"URL must contain competition name");

        let url = "https://www.soaringspot.com/en_gb/test/invalid/club/task-1-on-2025-01-01/daily";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"Unsupported URL format");

        let url = "https://www.soaringspot.com/en_gb/test/results/club/invalid-format";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"Unsupported URL format");

        let url = "https://www.soaringspot.com/en_gb/test/results/club/task-1-on-invalid-date";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"Failed to parse date 'invalid-date': strptime parsing failed: %Y failed: failed to parse year: invalid number, no digits found");
    }
}
