use crate::log_entry::LogEntry;
use std::io::BufRead;
use streaming_iterator::StreamingIterator;

pub struct LogEntryReader<R: BufRead> {
    reader: R,
    entry: LogEntry,
}

impl<R: BufRead> LogEntryReader<R> {
    pub fn new(reader: R) -> Self {
        LogEntryReader {
            reader,
            entry: LogEntry::new(),
        }
    }

    pub fn with_source(mut self, source: usize) -> Self {
        self.entry = self.entry.with_source(source);
        self
    }
}

impl<R: BufRead> StreamingIterator for LogEntryReader<R> {
    type Item = LogEntry;

    fn advance(&mut self) {
        self.entry.reset();
        while let Ok(bytes_read) = self
            .reader
            .read_until(b'\n', &mut self.entry.contents_mut())
        {
            // read until an empty line (might be '\n' or '\r\n')
            if bytes_read <= 2 {
                break;
            }
        }
    }

    fn get(&self) -> Option<&Self::Item> {
        if self.entry.contents().is_empty() {
            None
        } else {
            Some(&self.entry)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_entry_reader_single_line() {
        let input = b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\n";
        let mut reader = LogEntryReader::new(&input[..]);

        let entry = reader.next().unwrap();
        assert_eq!(
            entry.contents(),
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
            entry.contents(),
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
            entry.contents(),
            &b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\n\n"[..]);

        let entry = reader.next().unwrap();
        assert_eq!(
            entry.contents(),
            &b"-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line\n"[..]
        );

        assert_eq!(reader.next(), None);
    }
}
