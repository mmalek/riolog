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
            .and_then(|input| parse_timestamp(input))
    }
}

fn parse_timestamp(input: &[u8]) -> Option<NaiveDateTime> {
    let input = String::from_utf8_lossy(input);
    NaiveDateTime::parse_from_str(&input, "%F %T.%3f").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

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
