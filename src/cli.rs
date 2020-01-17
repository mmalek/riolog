use crate::error::Error::InvalidCliOptionValue;
use crate::log_entry::LogLevel;
use crate::result::Result;
use chrono::{NaiveDate, NaiveDateTime};
use clap::{App, Arg};
use std::path::PathBuf;

const ARG_FILE_NAME: &str = "file-name";
const ARG_COLOR: &str = "color";
const ARG_WRAP: &str = "wrap";
const ARG_OUTPUT: &str = "output";
const ARG_VALUES_TRUE: [&str; 3] = ["yes", "true", "on"];
const ARG_VALUES_FALSE: [&str; 3] = ["no", "false", "off"];
const ARG_TIME_FROM: &str = "from";
const ARG_TIME_TO: &str = "to";
const ARG_LEVEL: &str = "level";
const ARG_CONTAINS: &str = "contains";

pub struct Options {
    pub color_enabled: bool,
    pub wrap: bool,
    pub time_from: Option<NaiveDateTime>,
    pub time_to: Option<NaiveDateTime>,
    pub contains: Option<String>,
    pub min_level: Option<LogLevel>,
    pub input_file: PathBuf,
    pub output_file: Option<PathBuf>,
}

impl Options {
    pub fn read() -> Result<Self> {
        let matches = App::new("riolog")
            .version("1.0")
            .about("RIO log filter & viewer")
            .arg(
                Arg::with_name(ARG_FILE_NAME)
                    .help("path to a log file")
                    .index(1)
                    .required(true),
            )
            .arg(Arg::with_name(ARG_COLOR)
                .long(ARG_COLOR)
                .short("c")
                .value_name("yes/no")
                .help("turn on/off colorized output. Default: 'yes' for interactive mode, 'no' for file output mode (-o)"))
            .arg(Arg::with_name(ARG_WRAP)
                .long(ARG_WRAP)
                .value_name("yes/no")
                .help("wrap long lines in interactive mode. Default: 'no'"))
            .arg(Arg::with_name(ARG_OUTPUT)
                .long(ARG_OUTPUT)
                .short("o")
                .value_name("FILE")
                .help("write the log to the output file"))
            .arg(Arg::with_name(ARG_TIME_FROM)
                .long(ARG_TIME_FROM)
                .value_name("DATE_TIME")
                .help("show only entries later than provided date/time. Accepted formats: \"2020-01-10\", \"2020-01-10 18:00\", \"2020-01-10 18:33:19\""))
            .arg(Arg::with_name(ARG_TIME_TO)
                .long(ARG_TIME_TO)
                .value_name("DATE_TIME")
                .help("show only entries earlier than provided date/time. Accepted formats: \"2020-01-10\", \"2020-01-10 18:00\", \"2020-01-10 18:33:19\""))
                .arg(Arg::with_name(ARG_LEVEL)
                    .long(ARG_LEVEL)
                    .value_name("NAME")
                    .help("show only entries with equal or higher level. Allowed values: debug, info, warning, critical, fatal"))
            .arg(Arg::with_name(ARG_CONTAINS)
                .long(ARG_CONTAINS)
                .value_name("STRING")
                .help("show only entries containing given string. Search is case-sensitive"))
            .get_matches();

        let output_file = matches.value_of_os(ARG_OUTPUT).map(PathBuf::from);

        let color_enabled = matches
            .value_of(ARG_COLOR)
            .map(|input| parse_bool_arg(input).ok_or(InvalidCliOptionValue(ARG_COLOR)))
            .transpose()?
            .unwrap_or_else(|| output_file.is_none());

        let wrap = matches
            .value_of(ARG_WRAP)
            .map(|input| parse_bool_arg(input).ok_or(InvalidCliOptionValue(ARG_WRAP)))
            .transpose()?
            .unwrap_or(false);

        let time_from = matches
            .value_of(ARG_TIME_FROM)
            .map(|input| parse_date_time_arg(input).ok_or(InvalidCliOptionValue(ARG_TIME_FROM)))
            .transpose()?;

        let time_to = matches
            .value_of(ARG_TIME_TO)
            .map(|input| parse_date_time_arg(input).ok_or(InvalidCliOptionValue(ARG_TIME_TO)))
            .transpose()?;

        let min_level = matches
            .value_of(ARG_LEVEL)
            .map(|input| parse_level_arg(input).ok_or(InvalidCliOptionValue(ARG_LEVEL)))
            .transpose()?;

        let contains = matches.value_of(ARG_CONTAINS).map(String::from);

        let input_file = matches
            .value_of_os("file-name")
            .map(PathBuf::from)
            .expect("missing \"file-name\" parameter");

        Ok(Options {
            color_enabled,
            wrap,
            time_from,
            time_to,
            min_level,
            contains,
            input_file,
            output_file,
        })
    }

    pub fn is_filtering_or_coloring(&self) -> bool {
        self.color_enabled
            || self.time_from.is_some()
            || self.time_to.is_some()
            || self.min_level.is_some()
            || self.contains.is_some()
    }
}

fn parse_bool_arg(input: &str) -> Option<bool> {
    let value = input.to_lowercase();
    if ARG_VALUES_TRUE.iter().any(|&v| v == value) {
        Some(true)
    } else if ARG_VALUES_FALSE.iter().any(|&v| v == value) {
        Some(false)
    } else {
        None
    }
}

pub fn parse_level_arg(input: &str) -> Option<LogLevel> {
    match input.to_lowercase().as_str() {
        "debug" => Some(LogLevel::Debug),
        "info" => Some(LogLevel::Info),
        "warning" => Some(LogLevel::Warning),
        "critical" => Some(LogLevel::Critical),
        "fatal" => Some(LogLevel::Fatal),
        _ => None,
    }
}

pub fn parse_date_time_arg(input: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(&input, "%F %T.%3f")
        .or_else(|_| NaiveDateTime::parse_from_str(&input, "%F %T"))
        .or_else(|_| NaiveDateTime::parse_from_str(&input, "%F %R"))
        .or_else(|_| NaiveDate::parse_from_str(&input, "%F").map(|d| d.and_hms(0, 0, 0)))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn parse_date_time_arg_ymdhmsm() {
        assert_eq!(
            parse_date_time_arg("2020-01-10 18:33:19.244").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms_milli(18, 33, 19, 244)
        );
    }

    #[test]
    fn parse_date_time_arg_ymdhms() {
        assert_eq!(
            parse_date_time_arg("2020-01-10 18:33:19").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(18, 33, 19)
        );
    }

    #[test]
    fn parse_date_time_arg_ymdhm() {
        assert_eq!(
            parse_date_time_arg("2020-01-10 18:33").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(18, 33, 0)
        );
    }

    #[test]
    fn parse_date_time_arg_ymd() {
        assert_eq!(
            parse_date_time_arg("2020-01-10").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(0, 0, 0)
        );
    }
}
