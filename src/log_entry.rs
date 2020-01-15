use crate::date_time::parse_timestamp;
use crate::error::Error;
use crate::result::Result;
use chrono::NaiveDateTime;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Critical = 3,
    Fatal = 4,
}

pub struct LogEntry {
    pub contents: Vec<u8>,
}

impl LogEntry {
    pub fn level(&self) -> Option<LogLevel> {
        self.contents
            .iter()
            .position(|&c| c == b'-')
            .and_then(|pos| self.contents.get(pos + 1))
            .and_then(|level| match level {
                b'd' => Some(LogLevel::Debug),
                b'i' => Some(LogLevel::Info),
                b'w' => Some(LogLevel::Warning),
                b'c' => Some(LogLevel::Critical),
                b'f' => Some(LogLevel::Fatal),
                _ => None,
            })
    }

    pub fn timestamp(&self) -> Option<NaiveDateTime> {
        self.contents
            .iter()
            .position(|&c| c == b'>')
            .map(|pos| pos + 2)
            .and_then(|pos| self.contents.get(pos..pos + 23))
            .and_then(|input| parse_timestamp(input).ok())
    }
}

pub fn parse_level_cli_option(input: &str, arg_name: &'static str) -> Result<LogLevel> {
    match input.to_lowercase().as_str() {
        "debug" => Ok(LogLevel::Debug),
        "info" => Ok(LogLevel::Info),
        "warning" => Ok(LogLevel::Warning),
        "critical" => Ok(LogLevel::Critical),
        "fatal" => Ok(LogLevel::Fatal),
        _ => Err(Error::InvalidCliOptionValue(arg_name)),
    }
}
