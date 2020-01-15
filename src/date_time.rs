use crate::error::Error;
use crate::result::Result;
use chrono::{NaiveDate, NaiveDateTime};

pub fn parse_date_time_cli_option(input: &str, arg_name: &'static str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(&input, "%F %T.%3f")
        .or_else(|_| NaiveDateTime::parse_from_str(&input, "%F %T"))
        .or_else(|_| NaiveDateTime::parse_from_str(&input, "%F %R"))
        .or_else(|_| NaiveDate::parse_from_str(&input, "%F").map(|d| d.and_hms(0, 0, 0)))
        .map_err(|_| Error::InvalidCliOptionValue(arg_name))
}

pub fn parse_timestamp(input: &[u8]) -> Result<NaiveDateTime> {
    let input = String::from_utf8_lossy(input);
    NaiveDateTime::parse_from_str(&input, "%F %T.%3f")
        .map_err(|_| Error::CannotParseTimestamp(input.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn parse_date_time_cli_option_ymdhmsm() {
        assert_eq!(
            parse_date_time_cli_option("2020-01-10 18:33:19.244", "arg").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms_milli(18, 33, 19, 244)
        );
    }

    #[test]
    fn parse_date_time_cli_option_ymdhms() {
        assert_eq!(
            parse_date_time_cli_option("2020-01-10 18:33:19", "arg").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(18, 33, 19)
        );
    }

    #[test]
    fn parse_date_time_cli_option_ymdhm() {
        assert_eq!(
            parse_date_time_cli_option("2020-01-10 18:33", "arg").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(18, 33, 0)
        );
    }

    #[test]
    fn parse_date_time_cli_option_ymd() {
        assert_eq!(
            parse_date_time_cli_option("2020-01-10", "arg").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(0, 0, 0)
        );
    }

    #[test]
    fn parse_timestamp_simple_test() {
        assert_eq!(
            parse_timestamp(b"2020-01-10 18:33:19.244").unwrap(),
            NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 01, 10),
                NaiveTime::from_hms_milli(18, 33, 19, 244)
            )
        );
    }
}
