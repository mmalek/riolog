use crate::log_entry::LogEntry;
use std::io::BufRead;
use streaming_iterator::StreamingIterator;

pub struct LogEntryReader<R: BufRead> {
    reader: R,
    eol_seq_last: u8,
    eol_seq_len: usize,
    entry: LogEntry,
}

impl<R: BufRead> LogEntryReader<R> {
    pub fn new(reader: R, eol_seq: &'static [u8]) -> Self {
        LogEntryReader {
            reader,
            eol_seq_last: *eol_seq.last().expect("EOL sequence is empty"),
            eol_seq_len: eol_seq.len(),
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
            .read_until(self.eol_seq_last, &mut self.entry.contents_mut())
        {
            if bytes_read <= self.eol_seq_len {
                if bytes_read == 0 || self.entry.contents().len() > self.eol_seq_len {
                    break;
                } else {
                    self.entry.contents_mut().clear();
                }
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
    use rev_buf_reader::RevBufReader;
    use std::io::Cursor;

    const EOL_LF: &[u8] = b"\n";
    const EOL_CRLF: &[u8] = b"\r\n";
    const LOG_ENTRY_ONE_LINE: &[u8] =
        b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry";
    const LOG_ENTRY_ONE_LINE_LF: &[u8] =
        b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\n";
    const LOG_ENTRY_ONE_LINE_CRLF: &[u8] =
        b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\r\n";
    const LOG_ENTRY_TWO_LINES_LF: &[u8] = b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\n\n-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line\n\n";
    const LOG_ENTRY_TWO_LINES_CRLF: &[u8] = b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\r\n\r\n-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line\r\n\r\n";

    #[test]
    fn log_entry_reader_single_line_lf() {
        let reader = Cursor::new(LOG_ENTRY_ONE_LINE_LF);
        let mut reader = LogEntryReader::new(reader, EOL_LF);
        let entry = reader.next().unwrap();
        assert_eq!(entry.contents(), LOG_ENTRY_ONE_LINE_LF);
        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_single_line_crlf() {
        let reader = Cursor::new(LOG_ENTRY_ONE_LINE_CRLF);
        let mut reader = LogEntryReader::new(reader, EOL_CRLF);
        let entry = reader.next().unwrap();
        assert_eq!(entry.contents(), LOG_ENTRY_ONE_LINE_CRLF);
        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_single_line_no_final_eol() {
        let reader = Cursor::new(LOG_ENTRY_ONE_LINE);
        let mut reader = LogEntryReader::new(reader, EOL_LF);
        let entry = reader.next().unwrap();
        assert_eq!(entry.contents(), LOG_ENTRY_ONE_LINE);
        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_two_lines_lf() {
        let reader = Cursor::new(LOG_ENTRY_TWO_LINES_LF);
        let mut reader = LogEntryReader::new(reader, EOL_LF);

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            "-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\n\n");

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            "-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line\n\n"
        );

        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_two_lines_crlf() {
        let reader = Cursor::new(LOG_ENTRY_TWO_LINES_CRLF);
        let mut reader = LogEntryReader::new(reader, EOL_CRLF);

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            "-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\r\n\r\n");

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            "-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line\r\n\r\n"
        );

        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_single_line_rev_lf() {
        let reader = Cursor::new(LOG_ENTRY_ONE_LINE_LF);
        let reader = RevBufReader::new(reader);
        let mut reader = LogEntryReader::new(reader, EOL_LF);
        let entry = reader.next().unwrap();
        assert_eq!(entry.contents(), LOG_ENTRY_ONE_LINE_LF);
        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_two_lines_crlf_rev() {
        let reader = Cursor::new(LOG_ENTRY_TWO_LINES_CRLF);
        let reader = RevBufReader::new(reader);
        let mut reader = LogEntryReader::new(reader, EOL_CRLF);

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            "-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line\r\n"
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            "-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry\r\n\r\n"
        );

        assert_eq!(reader.next(), None);
    }
}
