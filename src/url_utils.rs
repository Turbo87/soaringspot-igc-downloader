use jiff::civil::Date;
use url::Url;

#[derive(Debug, Clone)]
pub struct DailyUrlInfo {
    pub competition: String,
    pub class: String,
    pub date: Date,
    pub task_number: u32,
}

impl DailyUrlInfo {
    /// Generates a daily result URL from the DailyUrlInfo
    pub fn to_daily_url(&self) -> String {
        let date_str = self.date.strftime("%Y-%m-%d").to_string();
        format!(
            "https://www.soaringspot.com/en_gb/{}/results/{}/task-{}-on-{}/daily",
            self.competition, self.class, self.task_number, date_str
        )
    }
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

pub fn extract_url_info(url: &Url) -> Result<UrlInfo, Box<dyn std::error::Error>> {
    // Validate the URL scheme
    if url.scheme() != "https" && url.scheme() != "http" {
        return Err("URL must use HTTP or HTTPS scheme".into());
    }

    // Validate the host
    let host = url.host_str().ok_or("Invalid URL - missing host")?;
    if host != "www.soaringspot.com" && host != "soaringspot.com" {
        return Err("URL must be from soaringspot.com or www.soaringspot.com".into());
    }

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

    // Extract task number and date from task-{n}-on-{date}
    let (task_part, date_str) = task
        .split_once("-on-")
        .ok_or("Could not extract date from task segment")?;

    // Extract task number from "task-{n}"
    let task_number_str = task_part
        .strip_prefix("task-")
        .ok_or("Task segment must start with 'task-'")?;

    let task_number = task_number_str
        .parse::<u32>()
        .map_err(|e| format!("Failed to parse task number '{}': {}", task_number_str, e))?;

    // Parse the date string using jiff
    let date = Date::strptime("%Y-%m-%d", date_str)
        .map_err(|e| format!("Failed to parse date '{}': {}", date_str, e))?;

    Ok(UrlInfo::Daily(DailyUrlInfo {
        competition,
        class,
        date,
        task_number,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                task_number: 10,
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
                task_number: 5,
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

        // Test without www prefix and with HTTP scheme
        let url = "http://soaringspot.com/en_gb/test/results";
        let url = Url::parse(url).unwrap();
        let info = extract_url_info(&url).unwrap();
        insta::assert_debug_snapshot!(info, @r#"
        Competition {
            competition: "test",
        }
        "#);

        // Test error cases
        let url = "ftp://www.soaringspot.com/en_gb";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"URL must use HTTP or HTTPS scheme");

        let url = "https://www.google.com";
        let url = Url::parse(url).unwrap();
        let result = extract_url_info(&url);
        insta::assert_snapshot!(result.unwrap_err(), @"URL must be from soaringspot.com or www.soaringspot.com");

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

    #[test]
    fn test_daily_url_info_to_daily_url() {
        // Test URL generation
        let daily_info = DailyUrlInfo {
            competition: "39th-fai-world-gliding-championships-tabor-2025".to_string(),
            class: "club".to_string(),
            date: Date::constant(2025, 6, 19),
            task_number: 10,
        };

        let url = daily_info.to_daily_url();
        insta::assert_snapshot!(url, @"https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025/results/club/task-10-on-2025-06-19/daily");

        // Test with different values
        let daily_info = DailyUrlInfo {
            competition: "test-competition".to_string(),
            class: "standard".to_string(),
            date: Date::constant(2024, 12, 1),
            task_number: 5,
        };

        let url = daily_info.to_daily_url();
        insta::assert_snapshot!(url, @"https://www.soaringspot.com/en_gb/test-competition/results/standard/task-5-on-2024-12-01/daily");
    }

    #[test]
    fn test_url_roundtrip() {
        // Test that we can parse a URL and generate the same URL back
        let original_url = "https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025/results/club/task-10-on-2025-06-19/daily";
        let parsed_url = Url::parse(original_url).unwrap();
        let url_info = extract_url_info(&parsed_url).unwrap();

        if let UrlInfo::Daily(daily_info) = url_info {
            let generated_url = daily_info.to_daily_url();
            assert_eq!(generated_url, original_url);
        } else {
            panic!("Expected Daily variant");
        }
    }
}
