use crate::log_entry::LogEntry;
use std::io::BufRead;

pub struct LogEntryReader<R: BufRead> {
    reader: R,
}

impl<R: BufRead> LogEntryReader<R> {
    pub fn new(reader: R) -> Self {
        LogEntryReader { reader }
    }
}

impl<R: BufRead> Iterator for LogEntryReader<R> {
    type Item = LogEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut contents = Vec::new();
        loop {
            if let Ok(bytes_read) = self.reader.read_until(b'\n', &mut contents) {
                // read until an empty line (might be '\n' or '\r\n')
                if bytes_read <= 2 {
                    break;
                }
            } else {
                return None;
            }
        }

        if contents.is_empty() {
            None
        } else {
            Some(LogEntry { contents })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn log_entry_reader_single_line() {
        let input = b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\n";
        let mut reader = LogEntryReader::new(&input[..]);

        let entry = reader.next().unwrap();
        assert_eq!(
            entry.contents.as_slice(),
            &b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\n"[..]);

        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_single_line_no_final_eol() {
        let input =
            b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry";
        let mut reader = LogEntryReader::new(&input[..]);

        let entry = reader.next().unwrap();
        assert_eq!(
            entry.contents.as_slice(),
            &b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry"
                [..]
        );

        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_two_lines() {
        let input =
            b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry

-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line
";
        let mut reader = LogEntryReader::new(&input[..]);

        let entry = reader.next().unwrap();
        assert_eq!(
            entry.contents.as_slice(),
            &b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\n\n"[..]);

        let entry = reader.next().unwrap();
        assert_eq!(
            entry.contents.as_slice(),
            &b"-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line\n"[..]
        );

        assert_eq!(reader.next(), None);
    }
}
