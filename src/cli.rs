use crate::error::Error;
use crate::log_entry::LogLevel;
use crate::result::Result;
use chrono::{NaiveDate, NaiveDateTime};
use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

const ARG_FILE_NAME: &str = "file-name";
const ARG_COLOR: &str = "color";
const ARG_OUTPUT: &str = "output";
const ARG_VALUES_TRUE: [&str; 3] = ["yes", "true", "on"];
const ARG_VALUES_FALSE: [&str; 3] = ["no", "false", "off"];
const ARG_TIME_FROM: &str = "from";
const ARG_TIME_TO: &str = "to";
const ARG_LEVEL: &str = "level";

pub struct Options {
    pub color_enabled: bool,
    pub time_from: Option<NaiveDateTime>,
    pub time_to: Option<NaiveDateTime>,
    pub min_level: Option<LogLevel>,
    pub input_file: PathBuf,
    pub output_file: Option<PathBuf>,
}

impl Options {
    pub fn read() -> Result<Self> {
        let matches = App::new("riolog")
            .version("0.1")
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
                .help("turn off colorized output. Default: 'yes' for interactive mode, 'no' for file output mode (-o)"))
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
            .get_matches();

        let output_file = matches.value_of_os(ARG_OUTPUT).map(PathBuf::from);

        let color_enabled =
            parse_bool_arg(&matches, ARG_COLOR)?.unwrap_or_else(|| output_file.is_none());

        let time_from = matches
            .value_of(ARG_TIME_FROM)
            .map(|input| parse_date_time_arg(input, ARG_TIME_FROM))
            .transpose()?;

        let time_to = matches
            .value_of(ARG_TIME_TO)
            .map(|input| parse_date_time_arg(input, ARG_TIME_TO))
            .transpose()?;

        let min_level = matches
            .value_of(ARG_LEVEL)
            .map(|input| parse_level_arg(input, ARG_TIME_TO))
            .transpose()?;

        let input_file = matches
            .value_of_os("file-name")
            .map(PathBuf::from)
            .expect("missing \"file-name\" parameter");

        Ok(Options {
            color_enabled,
            time_from,
            time_to,
            min_level,
            input_file,
            output_file,
        })
    }

    pub fn is_filtering_or_coloring(&self) -> bool {
        self.color_enabled
            || self.time_from.is_some()
            || self.time_to.is_some()
            || self.min_level.is_some()
    }
}

fn parse_bool_arg(matches: &ArgMatches, arg_name: &'static str) -> Result<Option<bool>> {
    if let Some(value) = matches.value_of(ARG_COLOR) {
        let value = value.to_lowercase();
        if ARG_VALUES_TRUE.iter().any(|&v| v == value) {
            Ok(Some(true))
        } else if ARG_VALUES_FALSE.iter().any(|&v| v == value) {
            Ok(Some(false))
        } else {
            Err(Error::InvalidCliOptionValue(arg_name))
        }
    } else {
        Ok(None)
    }
}

pub fn parse_level_arg(input: &str, arg_name: &'static str) -> Result<LogLevel> {
    match input.to_lowercase().as_str() {
        "debug" => Ok(LogLevel::Debug),
        "info" => Ok(LogLevel::Info),
        "warning" => Ok(LogLevel::Warning),
        "critical" => Ok(LogLevel::Critical),
        "fatal" => Ok(LogLevel::Fatal),
        _ => Err(Error::InvalidCliOptionValue(arg_name)),
    }
}

pub fn parse_date_time_arg(input: &str, arg_name: &'static str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(&input, "%F %T.%3f")
        .or_else(|_| NaiveDateTime::parse_from_str(&input, "%F %T"))
        .or_else(|_| NaiveDateTime::parse_from_str(&input, "%F %R"))
        .or_else(|_| NaiveDate::parse_from_str(&input, "%F").map(|d| d.and_hms(0, 0, 0)))
        .map_err(|_| Error::InvalidCliOptionValue(arg_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn parse_date_time_arg_ymdhmsm() {
        assert_eq!(
            parse_date_time_arg("2020-01-10 18:33:19.244", "arg").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms_milli(18, 33, 19, 244)
        );
    }

    #[test]
    fn parse_date_time_arg_ymdhms() {
        assert_eq!(
            parse_date_time_arg("2020-01-10 18:33:19", "arg").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(18, 33, 19)
        );
    }

    #[test]
    fn parse_date_time_arg_ymdhm() {
        assert_eq!(
            parse_date_time_arg("2020-01-10 18:33", "arg").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(18, 33, 0)
        );
    }

    #[test]
    fn parse_date_time_arg_ymd() {
        assert_eq!(
            parse_date_time_arg("2020-01-10", "arg").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms(0, 0, 0)
        );
    }
}
