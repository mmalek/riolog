use crate::result::Result;
use std::io::{Read, Seek, SeekFrom};

pub struct RevReader<R: Read> {
    reader: R,
    pos: u64,
    buf: Vec<u8>,
    buf_pos: usize,
}

impl<R: Read + Seek> RevReader<R> {
    pub fn with_capacity(mut inner: R, capacity: usize) -> Result<Self> {
        let pos = inner.seek(SeekFrom::End(0))?;

        let mut buf = Vec::new();
        buf.resize(capacity, 0);

        Ok(RevReader {
            reader: inner,
            pos,
            buf,
            buf_pos: 0,
        })
    }

    pub fn read_until(&mut self, byte: u8, skip_len: usize) -> Option<Vec<u8>> {
        let mut output = Vec::new();

        loop {
            if self.buf_pos == 0 {
                let bytes_to_read = if self.pos >= self.buf.len() as u64 {
                    self.buf.len()
                } else if self.pos > 0 {
                    self.pos as usize
                } else if !output.is_empty() {
                    return Some(output);
                } else {
                    return None;
                };

                self.pos -= bytes_to_read as u64;

                self.reader.seek(SeekFrom::Start(self.pos)).ok()?;
                self.reader
                    .read_exact(&mut self.buf[0..bytes_to_read])
                    .ok()?;
                self.reader.seek(SeekFrom::Start(self.pos)).ok()?;

                self.buf_pos = bytes_to_read;
            }

            let mut find_it = self.buf[0..self.buf_pos].iter().enumerate().rev();

            if let Some(i) = find_it.find(|(_, value)| **value == byte).map(|(i, _)| i) {
                Self::push_front(&mut output, &self.buf[(i + skip_len)..self.buf_pos]);
                self.buf_pos = i;
                return Some(output);
            }

            Self::push_front(&mut output, &self.buf[0..self.buf_pos]);
            self.buf_pos = 0;
        }
    }

    fn push_front(target: &mut Vec<u8>, source: &[u8]) {
        if source.is_empty() {
        } else if target.is_empty() {
            *target = source.to_vec();
        } else {
            *target = source.iter().chain(target.iter()).copied().collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rev_reader_lf() -> Result<()> {
        let input: &[u8] = b"first line\n\nthird line\nfourth line";
        let reader = Cursor::new(input);
        let mut reader = RevReader::with_capacity(reader, 1024)?;
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("fourth line".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("third line".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("first line".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            None
        );
        Ok(())
    }

    #[test]
    fn rev_reader_trailing_lf() -> Result<()> {
        let input: &[u8] = b"first line\n\nthird line\nfourth line\n";
        let reader = Cursor::new(input);
        let mut reader = RevReader::with_capacity(reader, 1024)?;
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("fourth line".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("third line".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).to_string()),
            Some("first line".to_string())
        );
        assert_eq!(
            reader
                .read_until(b'\n', 1)
                .map(|s| String::from_utf8_lossy(&s).as_ref().to_string()),
            None
        );
        Ok(())
    }
}
