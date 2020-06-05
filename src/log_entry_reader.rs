use crate::log_entry::LogEntry;
use crate::result::Result;
use crate::rev_reader::RevReader;
use std::io::{BufRead, Read, Seek};
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

pub struct LogEntryRevReader<R: Read> {
    reader: RevReader<R>,
    eol_seq_first: u8,
    eol_seq: &'static [u8],
    entry: LogEntry,
}

impl<R: Read + Seek> LogEntryRevReader<R> {
    pub fn with_capacity(reader: R, eol_seq: &'static [u8], capacity: usize) -> Result<Self> {
        Ok(LogEntryRevReader {
            reader: RevReader::with_capacity(reader, capacity)?,
            eol_seq_first: *eol_seq.first().expect("EOL sequence is empty"),
            eol_seq,
            entry: LogEntry::new(),
        })
    }

    pub fn with_source(mut self, source: usize) -> Self {
        self.entry = self.entry.with_source(source);
        self
    }
}

impl<R: Read + Seek> StreamingIterator for LogEntryRevReader<R> {
    type Item = LogEntry;

    fn advance(&mut self) {
        self.entry.reset();

        while let Some(buf) = self
            .reader
            .read_until(self.eol_seq_first, self.eol_seq.len())
        {
            if !buf.is_empty() && self.entry.contents_mut().is_empty() {
                *self.entry.contents_mut() = buf;
            } else if !buf.is_empty() && !self.entry.contents_mut().is_empty() {
                *self.entry.contents_mut() =
                    [buf.as_slice(), self.eol_seq, self.entry.contents()].concat();
            } else if buf.is_empty() && !self.entry.contents_mut().is_empty() {
                break;
            }
        }

        if !self.entry.contents_mut().is_empty() {
            self.entry.contents_mut().reserve(self.eol_seq.len() * 2);
            for c in self.eol_seq.iter().cycle().take(self.eol_seq.len() * 2) {
                self.entry.contents_mut().push(*c);
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
    use std::io::Cursor;

    const EOL_LF: &[u8] = b"\n";
    const EOL_CRLF: &[u8] = b"\r\n";
    const LOG_ENTRIES: [&[u8]; 2] = [
        b"-info:<16866> 2020-01-13 20:08:18.476 UTC [Category]: Contents of single line entry",
        b"-warning:<16866> 2020-01-13 20:09:18.476 UTC [Category]: The second line",
    ];
    const EXTRA_LOG_ENTRY_LINE: &[u8] = b"MESSAGE Alphabet";

    #[test]
    fn log_entry_reader_single_line_lf() {
        let one_line = [LOG_ENTRIES[0], EOL_LF].concat();
        let reader = Cursor::new(&one_line);
        let mut reader = LogEntryReader::new(reader, EOL_LF);
        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(one_line.as_slice())
        );
        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_single_line_crlf() {
        let one_line = [LOG_ENTRIES[0], EOL_CRLF].concat();
        let reader = Cursor::new(&one_line);
        let mut reader = LogEntryReader::new(reader, EOL_CRLF);
        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(one_line.as_slice())
        );
        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_single_line_no_final_eol() {
        let reader = Cursor::new(LOG_ENTRIES[0]);
        let mut reader = LogEntryReader::new(reader, EOL_LF);
        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(LOG_ENTRIES[0])
        );
        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_two_lines_lf() {
        let line_one = [LOG_ENTRIES[0], EOL_LF, EOL_LF].concat();
        let line_two = [LOG_ENTRIES[1], EOL_LF, EOL_LF].concat();

        let reader = Cursor::new([line_one.as_slice(), line_two.as_slice()].concat());
        let mut reader = LogEntryReader::new(reader, EOL_LF);

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_two.as_slice())
        );

        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_two_lines_extra_line_lf() {
        let line_one = [LOG_ENTRIES[0], EOL_LF, EOL_LF].concat();
        let line_two = [LOG_ENTRIES[1], EOL_LF, EXTRA_LOG_ENTRY_LINE, EOL_LF, EOL_LF].concat();

        let reader = Cursor::new([line_one.as_slice(), line_two.as_slice()].concat());
        let mut reader = LogEntryReader::new(reader, EOL_LF);

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_two.as_slice())
        );

        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_two_lines_crlf() {
        let line_one = [LOG_ENTRIES[0], EOL_CRLF, EOL_CRLF].concat();
        let line_two = [LOG_ENTRIES[1], EOL_CRLF, EOL_CRLF].concat();

        let reader = Cursor::new([line_one.as_slice(), line_two.as_slice()].concat());
        let mut reader = LogEntryReader::new(reader, EOL_CRLF);

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_two.as_slice())
        );

        assert_eq!(reader.next(), None);
    }

    #[test]
    fn log_entry_reader_single_line_rev_lf() -> Result<()> {
        let one_line = [LOG_ENTRIES[0], EOL_LF].concat();
        let reader = Cursor::new(&one_line);
        let mut reader = LogEntryRevReader::with_capacity(reader, EOL_LF, 1024 * 1024)?;
        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(&[LOG_ENTRIES[0], EOL_LF, EOL_LF].concat())
        );
        assert_eq!(reader.next(), None);
        Ok(())
    }

    #[test]
    fn log_entry_reader_single_line_rev_crlf() -> Result<()> {
        let one_line = [LOG_ENTRIES[0], EOL_CRLF].concat();
        let reader = Cursor::new(&one_line);
        let mut reader = LogEntryRevReader::with_capacity(reader, EOL_CRLF, 1024 * 1024)?;
        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(&[LOG_ENTRIES[0], EOL_CRLF, EOL_CRLF].concat())
        );
        assert_eq!(reader.next(), None);
        Ok(())
    }

    #[test]
    fn log_entry_reader_two_lines_lf_rev() -> Result<()> {
        let line_one = [LOG_ENTRIES[0], EOL_LF, EOL_LF].concat();
        let line_two = [LOG_ENTRIES[1], EOL_LF, EOL_LF].concat();

        let reader = Cursor::new([line_one.as_slice(), line_two.as_slice()].concat());
        let mut reader = LogEntryRevReader::with_capacity(reader, EOL_LF, 1024 * 1024)?;

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_two.as_slice())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        assert_eq!(reader.next(), None);
        Ok(())
    }

    #[test]
    fn log_entry_reader_two_lines_crlf_rev() -> Result<()> {
        let line_one = [LOG_ENTRIES[0], EOL_CRLF, EOL_CRLF].concat();
        let line_two = [LOG_ENTRIES[1], EOL_CRLF, EOL_CRLF].concat();

        let reader = Cursor::new([line_one.as_slice(), line_two.as_slice()].concat());
        let mut reader = LogEntryRevReader::with_capacity(reader, EOL_CRLF, 1024 * 1024)?;

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_two.as_slice())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        assert_eq!(reader.next(), None);
        Ok(())
    }

    #[test]
    fn log_entry_reader_two_lines_crlf_no_trailing_rev() -> Result<()> {
        let line_one = [LOG_ENTRIES[0], EOL_CRLF, EOL_CRLF].concat();
        let line_two = [LOG_ENTRIES[1], EOL_CRLF].concat();

        let reader = Cursor::new([line_one.as_slice(), line_two.as_slice()].concat());
        let mut reader = LogEntryRevReader::with_capacity(reader, EOL_CRLF, 1024 * 1024)?;

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(&[LOG_ENTRIES[1], EOL_CRLF, EOL_CRLF].concat())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        assert_eq!(reader.next(), None);
        Ok(())
    }

    #[test]
    fn log_entry_reader_two_lines_crlf_no_trailing2_rev() -> Result<()> {
        let line_one = [LOG_ENTRIES[0], EOL_CRLF, EOL_CRLF].concat();
        let line_two = LOG_ENTRIES[1];

        let reader = Cursor::new([line_one.as_slice(), line_two].concat());
        let mut reader = LogEntryRevReader::with_capacity(reader, EOL_CRLF, 1024 * 1024)?;

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(&[LOG_ENTRIES[1], EOL_CRLF, EOL_CRLF].concat())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        assert_eq!(reader.next(), None);
        Ok(())
    }

    #[test]
    fn log_entry_reader_two_lines_lf_small_buffer_rev() -> Result<()> {
        let line_one = [LOG_ENTRIES[0], EOL_LF, EOL_LF].concat();
        let line_two = [LOG_ENTRIES[1], EOL_LF, EOL_LF].concat();

        let reader = Cursor::new([line_one.as_slice(), line_two.as_slice()].concat());
        let mut reader = LogEntryRevReader::with_capacity(reader, EOL_LF, 10)?;

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_two.as_slice())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        assert_eq!(reader.next(), None);
        Ok(())
    }

    #[test]
    fn log_entry_reader_two_lines_lf_extra_line_rev() -> Result<()> {
        let line_one = [LOG_ENTRIES[0], EOL_LF, EOL_LF].concat();
        let line_two = [LOG_ENTRIES[1], EOL_LF, EXTRA_LOG_ENTRY_LINE, EOL_LF, EOL_LF].concat();

        let reader = Cursor::new([line_one.as_slice(), line_two.as_slice()].concat());
        let mut reader = LogEntryRevReader::with_capacity(reader, EOL_LF, 10)?;

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_two.as_slice())
        );

        let entry = reader.next().unwrap();
        assert_eq!(
            String::from_utf8_lossy(entry.contents()),
            String::from_utf8_lossy(line_one.as_slice())
        );

        assert_eq!(reader.next(), None);
        Ok(())
    }
}
