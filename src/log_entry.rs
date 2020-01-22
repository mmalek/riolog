use chrono::NaiveDateTime;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Critical = 3,
    Fatal = 4,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cache<T> {
    Empty,
    Filled(T),
}

#[derive(Debug, PartialEq)]
pub struct LogEntry {
    contents: Vec<u8>,
    level: Cell<Cache<Option<LogLevel>>>,
    timestamp: Cell<Cache<Option<NaiveDateTime>>>,
    source: usize, // index of log source the entry comes from
}

impl LogEntry {
    pub fn new(contents: Vec<u8>) -> LogEntry {
        LogEntry {
            contents,
            level: Cell::new(Cache::Empty),
            timestamp: Cell::new(Cache::Empty),
            source: 0,
        }
    }

    pub fn with_source(mut self, source: usize) -> Self {
        self.source = source;
        self
    }

    pub fn contents(&self) -> &[u8] {
        self.contents.as_slice()
    }

    pub fn level(&self) -> Option<LogLevel> {
        if let Cache::Filled(level) = self.level.get() {
            level
        } else {
            let level = self
                .contents
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
                });

            self.level.set(Cache::Filled(level));
            level
        }
    }

    pub fn timestamp(&self) -> Option<NaiveDateTime> {
        if let Cache::Filled(timestamp) = self.timestamp.get() {
            timestamp
        } else {
            let timestamp = self
                .contents
                .iter()
                .position(|&c| c == b'>')
                .map(|pos| pos + 2)
                .and_then(|pos| self.contents.get(pos..pos + 23))
                .and_then(|input| parse_timestamp(input));

            self.timestamp.set(Cache::Filled(timestamp));
            timestamp
        }
    }

    pub fn source(&self) -> usize {
        self.source
    }
}

fn parse_timestamp(input: &[u8]) -> Option<NaiveDateTime> {
    let input = String::from_utf8_lossy(input);
    NaiveDateTime::parse_from_str(&input, "%F %T.%3f").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn parse_timestamp_simple_test() {
        assert_eq!(
            parse_timestamp(b"2020-01-10 18:33:19.244").unwrap(),
            NaiveDate::from_ymd(2020, 01, 10).and_hms_milli(18, 33, 19, 244)
        );
    }

    #[test]
    fn log_entry_level() {
        let entry = LogEntry::new(b"-info:<16866> 2020-01-01 20:00:00.000 UTC [A]: B".to_vec());
        assert_eq!(entry.level(), Some(LogLevel::Info));
    }

    #[test]
    fn log_entry_timestamp() {
        let entry = LogEntry::new(b"-info:<16866> 2020-01-01 20:00:00.000 UTC [A]: B".to_vec());
        assert_eq!(
            entry.timestamp(),
            Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(20, 0, 0))
        );
    }
}
