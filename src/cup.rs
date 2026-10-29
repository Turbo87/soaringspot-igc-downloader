//! Converts SoaringSpot task data into the SeeYou CUP format.

use jiff::civil::{DateTime, Time};
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt::Write;

const WAYPOINT_HEADER: &str = "name,code,country,lat,lon,elev,style,rwdir,rwlen,rwwidth,freq,desc";

/// The task data embedded in a SoaringSpot task page.
#[derive(Debug, Deserialize)]
pub struct Task {
    task_name: Option<String>,
    no_start: DateTime,
    /// Minimum task time of assigned area tasks in seconds, `0` otherwise.
    task_duration: u32,
    start_on_entry: bool,
    task_points: Vec<TaskPoint>,
}

#[derive(Debug, Clone, Deserialize)]
struct TaskPoint {
    name: String,
    latitude: f64,
    longitude: f64,
    elevation: f64,
    multiple_start: bool,
    #[serde(flatten)]
    zone: Zone,
}

/// Observation zone of a task point. Angles are in radians.
#[derive(Debug, Clone, Deserialize)]
struct Zone {
    oz_type: ZoneDirection,
    oz_radius1: f64,
    oz_angle1: f64,
    oz_radius2: f64,
    oz_angle2: f64,
    oz_angle12: f64,
    oz_line: bool,
    oz_reduce: bool,
    oz_move: bool,
    oz_max_altitude: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ZoneDirection {
    Fixed,
    Symmetric,
    Next,
    Previous,
    Start,
}

impl Zone {
    /// Returns the `ObsZone` parameters after the point index.
    fn cup_parameters(&self) -> String {
        let style = match self.oz_type {
            ZoneDirection::Fixed => 0,
            ZoneDirection::Symmetric => 1,
            ZoneDirection::Next => 2,
            ZoneDirection::Previous => 3,
            ZoneDirection::Start => 4,
        };
        let (r1, a1) = (self.oz_radius1, degrees(self.oz_angle1));
        let mut parameters = format!("Style={style},R1={r1}m,A1={a1}");
        if self.oz_radius2 != 0.0 {
            let (r2, a2) = (self.oz_radius2, degrees(self.oz_angle2));
            parameters += &format!(",R2={r2}m,A2={a2}");
        }
        if let ZoneDirection::Fixed = self.oz_type {
            parameters += &format!(",A12={}", degrees(self.oz_angle12));
        }
        if self.oz_line {
            parameters += ",Line=1";
        }
        if self.oz_reduce {
            parameters += ",Reduce=1";
        }
        if self.oz_max_altitude != 0.0 {
            parameters += &format!(",MaxAlt={}m", self.oz_max_altitude);
        }
        parameters
    }
}

/// Generates a CUP file with the waypoints and the task, together with
/// warnings about task data that the CUP file can't represent.
pub fn task_cup(task: &Task) -> Result<(String, Vec<String>), Box<dyn std::error::Error>> {
    let points = task.task_points.iter();
    let (alternative_starts, sequence): (Vec<_>, Vec<_>) =
        points.partition(|point| point.multiple_start);
    let [start, .., finish] = sequence[..] else {
        return Err("Task needs at least a start and a finish point".into());
    };

    let mut warnings = vec![];
    if task.start_on_entry {
        warnings.push("task.cup ignores start_on_entry".to_string());
    }

    let mut cup = format!("{WAYPOINT_HEADER}\n");
    let mut names = HashSet::new();
    for point in &task.task_points {
        if !names.insert(&point.name) {
            continue;
        }
        let name = quote(&point.name);
        let latitude = coordinate(point.latitude, 2, ['N', 'S']);
        let longitude = coordinate(point.longitude, 3, ['E', 'W']);
        let elevation = point.elevation;
        writeln!(cup, "{name},,,{latitude},{longitude},{elevation}m,1,,,,,")?;
    }

    writeln!(cup, "-----Related Tasks-----")?;
    let mut columns = vec![quote(task.task_name.as_deref().unwrap_or_default())];
    columns.push(quote(&start.name));
    columns.extend(sequence.iter().map(|point| quote(&point.name)));
    columns.push(quote(&finish.name));
    writeln!(cup, "{}", columns.join(","))?;

    let mut options = String::new();
    let no_start = task.no_start.time();
    if no_start != Time::midnight() {
        write!(options, ",NoStart={}", no_start.strftime("%H:%M:%S"))?;
    }
    let task_time = task.task_duration;
    if task_time != 0 {
        let (hours, minutes, seconds) = (task_time / 3600, task_time / 60 % 60, task_time % 60);
        write!(options, ",TaskTime={hours:02}:{minutes:02}:{seconds:02}")?;
    }
    if !options.is_empty() {
        writeln!(cup, "Options{options}")?;
    }

    for (index, point) in sequence.iter().enumerate() {
        writeln!(cup, "ObsZone={index},{}", point.zone.cup_parameters())?;
        if point.zone.oz_move {
            warnings.push(format!("task.cup ignores oz_move of {}", point.name));
        }
    }

    if !alternative_starts.is_empty() {
        let starts = std::iter::once(start).chain(alternative_starts.iter().copied());
        let names: Vec<_> = starts.map(|point| point.name.as_str()).collect();
        writeln!(cup, "STARTS={}", names.join(","))?;
    }
    for alternative in alternative_starts {
        if alternative.zone.cup_parameters() != start.zone.cup_parameters() {
            let name = &alternative.name;
            warnings.push(format!("task.cup uses the start zone for {name}"));
        }
    }

    Ok((cup, warnings))
}

fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// Formats an angle in radians as degrees and minutes, for example `5107.830N`.
fn coordinate(radians: f64, degree_digits: usize, hemispheres: [char; 2]) -> String {
    let thousandths = (radians.to_degrees().abs() * 60_000.0).round() as u64;
    let whole_degrees = thousandths / 60_000;
    let minutes = thousandths / 1000 % 60;
    let fraction = thousandths % 1000;
    let hemisphere = hemispheres[usize::from(radians < 0.0)];
    format!("{whole_degrees:0degree_digits$}{minutes:02}.{fraction:03}{hemisphere}")
}

/// Converts radians to degrees, rounded to remove the rounding errors of the
/// published values, so that a half circle becomes exactly `180`.
fn degrees(radians: f64) -> f64 {
    (radians.to_degrees() * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::extract_task_json;

    fn racing_task() -> Task {
        let html = include_str!("../tests/fixtures/task.html");
        serde_json::from_str(extract_task_json(html).unwrap()).unwrap()
    }

    #[test]
    fn test_racing_task() {
        let (cup, warnings) = task_cup(&racing_task()).unwrap();
        insta::assert_snapshot!(cup);
        insta::assert_debug_snapshot!(warnings, @"[]");
    }

    #[test]
    fn test_assigned_area_task() {
        let json = include_str!("../tests/fixtures/task-aat.json");
        let (cup, warnings) = task_cup(&serde_json::from_str(json).unwrap()).unwrap();
        insta::assert_snapshot!(cup);
        insta::assert_debug_snapshot!(warnings, @r#"
        [
            "task.cup ignores oz_move of 249 Finish South",
        ]
        "#);
    }

    #[test]
    fn test_multiple_starts() {
        let mut task = racing_task();
        let start = &task.task_points[0];
        let mut alternative = TaskPoint {
            name: "ALTSTART".to_string(),
            latitude: start.latitude + 0.001,
            multiple_start: true,
            ..start.clone()
        };
        task.task_points.push(alternative.clone());
        alternative.name = "WIDESTART".to_string();
        alternative.zone.oz_radius1 = 10000.0;
        task.task_points.push(alternative);

        let (cup, warnings) = task_cup(&task).unwrap();
        insta::assert_snapshot!(cup);
        insta::assert_debug_snapshot!(warnings, @r#"
        [
            "task.cup uses the start zone for WIDESTART",
        ]
        "#);
    }

    #[test]
    fn test_start_on_entry() {
        let task = Task {
            start_on_entry: true,
            ..racing_task()
        };
        let (_, warnings) = task_cup(&task).unwrap();
        insta::assert_debug_snapshot!(warnings, @r#"
        [
            "task.cup ignores start_on_entry",
        ]
        "#);
    }

    #[test]
    fn test_task_without_finish() {
        let mut task = racing_task();
        task.task_points.truncate(1);
        insta::assert_snapshot!(task_cup(&task).unwrap_err(), @"Task needs at least a start and a finish point");
    }

    #[test]
    fn test_coordinate_rounding() {
        let latitude = -(10.0_f64 + 59.9999 / 60.0).to_radians();
        insta::assert_snapshot!(coordinate(latitude, 2, ['N', 'S']), @"1100.000S");
    }
}
