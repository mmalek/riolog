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
