//! Converts the table of a SoaringSpot daily results page into CSV.

use scraper::{CaseSensitivity, ElementRef, Html, Selector};
use serde::Serialize;
use std::error::Error;

/// One table row of `results.csv`. The field names are the column names.
#[derive(Debug, Default, Serialize)]
struct ResultRow {
    rank: String,
    cn: String,
    contestant: String,
    countries: String,
    club: String,
    team: String,
    glider: String,
    handicap: String,
    start_time_local: String,
    finish_time_local: String,
    duration: String,
    speed_kmh: String,
    distance_km: String,
    points: String,
    points_sort_key: String,
    penalty_points: String,
    remarks: String,
}

/// Generates the content of `results.csv` from a daily results page, or
/// `None` if SoaringSpot has no results for the day yet.
pub fn results_csv(html: &str) -> Result<Option<String>, Box<dyn Error>> {
    let document = Html::parse_document(html);
    let table_selector = Selector::parse("table.result-daily")?;
    let Some(table) = document.select(&table_selector).next() else {
        if html.contains("No Results yet for this task.") {
            return Ok(None);
        }
        return Err("Results table not found on daily results page".into());
    };

    let header_selector = Selector::parse("thead th")?;
    let headers: Vec<_> = table.select(&header_selector).map(text).collect();
    let cell_selector = Selector::parse("td")?;
    let mut writer = csv::Writer::from_writer(vec![]);
    for row in table.select(&Selector::parse("tbody tr")?) {
        let cells: Vec<_> = row.select(&cell_selector).collect();
        if cells.len() != headers.len() {
            return Err("Results table row doesn't match the table header".into());
        }

        let mut record = ResultRow::default();
        for (header, cell) in headers.iter().zip(cells) {
            let text = text(cell);
            match header.as_str() {
                "#" => record.rank = text.strip_suffix('.').unwrap_or(&text).to_string(),
                "OP" => {}
                "CN" => record.cn = text,
                "Contestant" => {
                    record.contestant = text;
                    record.countries = countries(cell)?;
                }
                "Club" => record.club = text,
                "Team" => record.team = text,
                "Glider" => record.glider = text,
                "Handicap" => record.handicap = text,
                "Start" => record.start_time_local = clock_time(cell)?,
                "Finish" => record.finish_time_local = clock_time(cell)?,
                "Time" => record.duration = duration(cell)?,
                "Speed" => record.speed_kmh = converted_value(cell, |speed| speed * 3.6)?,
                "Distance" => record.distance_km = converted_value(cell, |metres| metres / 1000.0)?,
                "Points" => {
                    record.points = text.replace(',', "").parse::<f64>()?.to_string();
                    record.points_sort_key = data_value(cell)?.to_string();
                    (record.remarks, record.penalty_points) = remark(cell)?;
                }
                _ => return Err(format!("Unexpected column {header:?} in results table").into()),
            }
        }
        writer.serialize(record)?;
    }

    Ok(Some(String::from_utf8(writer.into_inner()?)?))
}

/// Returns the text of an element with all whitespace collapsed.
fn text(element: ElementRef) -> String {
    let words: Vec<_> = element.text().flat_map(str::split_whitespace).collect();
    words.join(" ")
}

/// Returns the space-separated country codes of the flags in a cell.
fn countries(cell: ElementRef) -> Result<String, Box<dyn Error>> {
    let flag_selector = Selector::parse("div.flag")?;
    let flags = cell.select(&flag_selector);
    let classes = flags.flat_map(|flag| flag.value().classes());
    let codes: Vec<_> = classes
        .filter_map(|class| class.strip_prefix("flag-"))
        .collect();
    Ok(codes.join(" "))
}

fn data_value<'a>(cell: ElementRef<'a>) -> Result<&'a str, Box<dyn Error>> {
    let value = cell.value().attr("data-value");
    Ok(value.ok_or("Results table cell without data-value")?)
}

/// Returns the precise value of a cell, or `None` if the cell is empty.
///
/// Empty cells still have a `data-value`, but it is meaningless.
fn precise_value<'a>(cell: ElementRef<'a>) -> Result<Option<&'a str>, Box<dyn Error>> {
    match text(cell).is_empty() {
        true => Ok(None),
        false => data_value(cell).map(Some),
    }
}

/// Formats the time of day of a `YYYYMMDDhhmmss` value as `hh:mm:ss`.
///
/// The date part is ignored, because SoaringSpot publishes wrong dates.
fn clock_time(cell: ElementRef) -> Result<String, Box<dyn Error>> {
    let Some(value) = precise_value(cell)? else {
        return Ok(String::new());
    };
    if value.len() != 14 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("Unexpected time value {value:?}").into());
    }
    let (hours, minutes, seconds) = (&value[8..10], &value[10..12], &value[12..]);
    Ok(format!("{hours}:{minutes}:{seconds}"))
}

/// Formats a value in seconds as `h:mm:ss`.
fn duration(cell: ElementRef) -> Result<String, Box<dyn Error>> {
    let Some(value) = precise_value(cell)? else {
        return Ok(String::new());
    };
    let seconds: u32 = value.parse()?;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    Ok(format!("{hours}:{minutes:02}:{seconds:02}"))
}

fn converted_value(cell: ElementRef, convert: fn(f64) -> f64) -> Result<String, Box<dyn Error>> {
    let Some(value) = precise_value(cell)? else {
        return Ok(String::new());
    };
    Ok(convert(value.parse()?).to_string())
}

/// Returns the text of the warning icon in a points cell, together with the
/// penalty points if the icon marks a penalty.
fn remark(cell: ElementRef) -> Result<(String, String), Box<dyn Error>> {
    let icons: Vec<_> = cell.select(&Selector::parse("i.fa-warning")?).collect();
    let icon = match icons[..] {
        [] => return Ok(Default::default()),
        [icon] => icon.value(),
        _ => return Err("Points cell with more than one warning icon".into()),
    };

    let remark = icon.attr("data-content").unwrap_or_default().to_string();
    if !icon.has_class("penalty-points", CaseSensitivity::CaseSensitive) {
        return Ok((remark, String::new()));
    }

    let points = match remark.strip_suffix(" penalty points)") {
        Some(rest) => rest.rsplit_once('(').map(|(_, points)| points),
        None => remark.strip_suffix(" penalty points"),
    };
    let penalty_points = points
        .and_then(|points| points.replace(',', "").parse::<f64>().ok())
        .ok_or_else(|| format!("Unexpected penalty {remark:?}"))?;
    Ok((remark, penalty_points.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_results_csv() {
        let html = include_str!("../tests/fixtures/day.html");
        insta::assert_snapshot!(results_csv(html).unwrap().unwrap());
    }

    #[test]
    fn test_results_csv_with_club_and_penalties() {
        let html = include_str!("../tests/fixtures/day-penalties.html");
        insta::assert_snapshot!(results_csv(html).unwrap().unwrap());
    }

    #[test]
    fn test_results_csv_with_team() {
        let html = include_str!("../tests/fixtures/day-team.html");
        let csv = results_csv(html).unwrap().unwrap();
        let lines: Vec<_> = csv.lines().take(3).collect();
        insta::assert_snapshot!(lines.join("\n"), @"
        rank,cn,contestant,countries,club,team,glider,handicap,start_time_local,finish_time_local,duration,speed_kmh,distance_km,points,points_sort_key,penalty_points,remarks
        1,1F,Pilot 1,de,,BB,LS1-f,1.009,13:20:28,16:34:03,3:13:35,87.1924095153828,281.316625,1000,10000024.50143,,
        2,SF,Pilot 2,de,,BY,LS3 WL,1.049,13:15:32,16:23:48,3:08:16,89.65473403930561,281.316625,979,9790025.1854094,,
        ");
    }

    #[test]
    fn test_results_csv_with_two_flags() {
        let html = include_str!("../tests/fixtures/day.html").replacen(
            r#"<div class="flag flag-cz" title="cz"></div>"#,
            r#"<div class="flag flag-cz" title="cz"></div><div class="flag flag-sk" title="sk"></div>"#,
            1,
        );
        let csv = results_csv(&html).unwrap().unwrap();
        insta::assert_snapshot!(csv.lines().nth(2).unwrap(), @"2,A3,Jane Doe,cz sk,,,ASW 20,1.071,12:24:49,16:42:34,4:17:45,100.8722557067868,433.33040625,991,9910028.4534014,,");
    }

    #[test]
    fn test_results_csv_without_results() {
        let html = "<div>\n    No Results yet for this task.\n</div>";
        insta::assert_debug_snapshot!(results_csv(html).unwrap(), @"None");
    }

    #[test]
    fn test_results_csv_without_table() {
        let html = include_str!("../tests/fixtures/task.html");
        insta::assert_snapshot!(results_csv(html).unwrap_err(), @"Results table not found on daily results page");
    }

    #[test]
    fn test_results_csv_with_unexpected_column() {
        let html =
            include_str!("../tests/fixtures/day.html").replace("<th>CN</th>", "<th>Callsign</th>");
        insta::assert_snapshot!(results_csv(&html).unwrap_err(), @r#"Unexpected column "Callsign" in results table"#);
    }

    #[test]
    fn test_results_csv_with_penalty_without_reason() {
        let html = include_str!("../tests/fixtures/day-penalties.html").replacen(
            "Outside Competition Area (1,000 penalty points)",
            "1,000 penalty points",
            1,
        );
        let csv = results_csv(&html).unwrap().unwrap();
        let row = csv
            .lines()
            .find(|line| line.contains("Drew Stone"))
            .unwrap();
        insta::assert_snapshot!(row, @r#"9,WUD,Drew Stone,,Adelaide University Gliding Club,,LS 3,1074,12:34:19,16:08:41,3:34:22,120.3138431037144,429.854625,-183,-1829966.1496334,1000,"1,000 penalty points""#);
    }

    #[test]
    fn test_results_csv_with_unexpected_penalty() {
        let html = include_str!("../tests/fixtures/day-penalties.html").replacen(
            "(1,000 penalty points)",
            "(lots of penalty points)",
            1,
        );
        insta::assert_snapshot!(results_csv(&html).unwrap_err(), @r#"Unexpected penalty "Outside Competition Area (lots of penalty points)""#);
    }
}
