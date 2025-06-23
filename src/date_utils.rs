use jiff::civil::Date;

/// Converts a jiff Date to a three-character IGC filename date format.
///
/// Format: `{last_digit_of_year}{month}{day_character}`
///
/// where `day_character` is:
/// - Digits 1-9 for days 1-9
/// - Letters A-V for days 10-31 (A=10, B=11, ..., J=19, ..., V=31)
pub fn date_to_igc_filename_prefix(date: Date) -> String {
    let last_digit_of_year = (date.year() % 10) as u8;
    let month = date.month() as u8;
    let day = date.day() as u8;

    let day_character = match day {
        1..=9 => char::from(b'0' + day),
        10..=31 => char::from(b'A' + day - 10),
        _ => unreachable!("Invalid day of month: {}", day),
    };

    format!("{last_digit_of_year}{month}{day_character}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_to_igc_filename() {
        let date = Date::constant(2025, 6, 19);
        assert_eq!(date_to_igc_filename_prefix(date), "56J");

        // Test numeric days (1-9)
        let date = Date::constant(2024, 1, 1);
        assert_eq!(date_to_igc_filename_prefix(date), "411");

        let date = Date::constant(2023, 12, 9);
        assert_eq!(date_to_igc_filename_prefix(date), "3129");

        // Test letter days (10-31)
        let date = Date::constant(2022, 7, 10);
        assert_eq!(date_to_igc_filename_prefix(date), "27A");

        let date = Date::constant(2021, 8, 15);
        assert_eq!(date_to_igc_filename_prefix(date), "18F");

        let date = Date::constant(2020, 3, 26);
        assert_eq!(date_to_igc_filename_prefix(date), "03Q");

        let date = Date::constant(2019, 5, 31);
        assert_eq!(date_to_igc_filename_prefix(date), "95V");
    }
}
