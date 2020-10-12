use crate::error::Error::InvalidCliOptionValue;
use crate::log_entry::LogLevel;
use crate::result::Result;
use chrono::{NaiveDate, NaiveDateTime};
use clap::{App, Arg, crate_version};
use std::path::PathBuf;

const ARG_FILE_NAMES: &str = "FILE";
const ARG_COLOR: &str = "color";
const ARG_FORMATTING: &str = "formatting";
const ARG_PAGER: &str = "pager";
const ARG_WRAP: &str = "wrap";
const ARG_OUTPUT: &str = "output";
const ARG_VALUES_TRUE: [&str; 3] = ["yes", "true", "on"];
const ARG_VALUES_FALSE: [&str; 3] = ["no", "false", "off"];
const ARG_SINCE: &str = "since";
const ARG_UNTIL: &str = "until";
const ARG_LEVEL: &str = "level";
const ARG_CONTAINS: &str = "contains";
const ARG_REVERSE: &str = "reverse";

#[derive(Clone)]
pub struct Options {
    pub color_enabled: bool,
    pub formatting_enabled: bool,
    pub pager: bool,
    pub wrap: bool,
    pub reverse: bool,
    pub filtering_options: FilteringOptions,
    pub input_files: Vec<PathBuf>,
    pub output_file: Option<PathBuf>,
}

#[derive(Clone)]
pub struct FilteringOptions {
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
    pub contains: Option<String>,
    pub min_level: Option<LogLevel>,
}

impl Options {
    pub fn read() -> Result<Self> {
        let matches = App::new("riolog")
            .version(crate_version!())
            .about("RIO log filter & viewer")
            .arg(
                Arg::with_name(ARG_FILE_NAMES)
                    .help("path to a log file(s). With no FILE, or when FILE is -, read standard input")
                    .index(1)
                    .multiple(true),
            )
            .arg(Arg::with_name(ARG_COLOR)
                .long(ARG_COLOR)
                .short("c")
                .value_name("BOOLEAN")
                .help("turn on/off colorized output. Default: enabled for interactive mode, disabled for file output mode (-o)"))
            .arg(Arg::with_name(ARG_FORMATTING)
                .long(ARG_FORMATTING)
                .value_name("BOOLEAN")
                .help("turn on/off special characters formatting. Default: true"))
            .arg(Arg::with_name(ARG_PAGER)
                .long(ARG_PAGER)
                .value_name("BOOLEAN")
                .help("turn on/off pager (using \"less\") for showing log. Default: true"))
            .arg(Arg::with_name(ARG_WRAP)
                .long(ARG_WRAP)
                .short("w")
                .help("wrap long lines in interactive mode"))
            .arg(Arg::with_name(ARG_REVERSE)
                .long(ARG_REVERSE)
                .short("r")
                .help("reverse output so that the newest entries are displayed first"))
            .arg(Arg::with_name(ARG_OUTPUT)
                .long(ARG_OUTPUT)
                .short("o")
                .value_name("FILE")
                .help("write the log to the output file"))
            .arg(Arg::with_name(ARG_SINCE)
                .long(ARG_SINCE)
                .short("S")
                .value_name("DATE_TIME")
                .help("show only entries later than provided date/time. Accepted formats: \"2020-01-10\", \"2020-01-10 18:00\", \"2020-01-10 18:33:19\""))
            .arg(Arg::with_name(ARG_UNTIL)
                .long(ARG_UNTIL)
                .short("U")
                .value_name("DATE_TIME")
                .help("show only entries earlier than provided date/time. Accepted formats: \"2020-01-10\", \"2020-01-10 18:00\", \"2020-01-10 18:33:19\""))
            .arg(Arg::with_name(ARG_LEVEL)
                .long(ARG_LEVEL)
                .short("L")
                .value_name("NAME")
                .help("show only entries with equal or higher level. Allowed values: debug, info, warning, critical, fatal"))
            .arg(Arg::with_name(ARG_CONTAINS)
                .long(ARG_CONTAINS)
                .short("C")
                .value_name("STRING")
                .help("show only entries containing given string. Search is case-sensitive"))
            .get_matches();

        let output_file = matches.value_of_os(ARG_OUTPUT).map(PathBuf::from);

        let color_enabled = matches
            .value_of(ARG_COLOR)
            .map(|input| parse_bool_arg(input).ok_or(InvalidCliOptionValue(ARG_COLOR)))
            .transpose()?
            .unwrap_or_else(|| output_file.is_none());

        let formatting_enabled = matches
            .value_of(ARG_FORMATTING)
            .map(|input| parse_bool_arg(input).ok_or(InvalidCliOptionValue(ARG_FORMATTING)))
            .transpose()?
            .unwrap_or(true);

        let pager = matches
            .value_of(ARG_PAGER)
            .map(|input| parse_bool_arg(input).ok_or(InvalidCliOptionValue(ARG_PAGER)))
            .transpose()?
            .unwrap_or(true);

        let wrap = matches.is_present(ARG_WRAP);

        let reverse = matches.is_present(ARG_REVERSE);

        let since = matches
            .value_of(ARG_SINCE)
            .map(|input| parse_date_time_arg(input).ok_or(InvalidCliOptionValue(ARG_SINCE)))
            .transpose()?;

        let until = matches
            .value_of(ARG_UNTIL)
            .map(|input| parse_date_time_arg(input).ok_or(InvalidCliOptionValue(ARG_UNTIL)))
            .transpose()?;

        let min_level = matches
            .value_of(ARG_LEVEL)
            .map(|input| parse_level_arg(input).ok_or(InvalidCliOptionValue(ARG_LEVEL)))
            .transpose()?;

        let contains = matches.value_of(ARG_CONTAINS).map(String::from);

        let input_files = matches
            .values_of_os(ARG_FILE_NAMES)
            .map(|os_values| os_values.map(PathBuf::from).collect())
            .unwrap_or_else(Vec::new);

        let filtering_options = FilteringOptions {
            since,
            until,
            min_level,
            contains,
        };

        Ok(Options {
            color_enabled,
            formatting_enabled,
            pager,
            wrap,
            reverse,
            filtering_options,
            input_files,
            output_file,
        })
    }

    pub fn is_filtering_or_coloring(&self) -> bool {
        self.color_enabled
            || self.filtering_options.since.is_some()
            || self.filtering_options.until.is_some()
            || self.filtering_options.min_level.is_some()
            || self.filtering_options.contains.is_some()
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
