mod error;
mod result;

use crate::error::Error;
use crate::result::Result;
use clap::{App, Arg};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn main() -> Result<()> {
    let matches = App::new("riolog")
        .version("0.1")
        .about("Hi-Fi log parser")
        .arg(
            Arg::with_name("file-name")
                .help("path to a log file")
                .index(1)
                .required(true),
        )
        .get_matches();

    let mut less_process = Command::new("less")
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let filename = matches
        .value_of("file-name")
        .expect("missing \"file-name\" parameter");

    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut less_stdin = less_process
        .stdin
        .as_mut()
        .ok_or(Error::CannotUseLessStdin)?;

    match format_and_copy(&mut reader, &mut less_stdin) {
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::BrokenPipe => (),
        Err(error) => return Err(error),
        Ok(()) => (),
    };

    less_process.wait()?;

    Ok(())
}

fn format_and_copy(reader: &mut impl BufRead, writer: &mut impl Write) -> Result<()> {
    let mut ctr_char_is_next = false;

    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }

        let mut empty_slice = false;

        for s in buf.split(|&c| c == b'\\') {
            if s.is_empty() {
                if ctr_char_is_next {
                    writer.write_all(b"\\")?;
                    empty_slice = true;
                    ctr_char_is_next = false;
                } else {
                    empty_slice = false;
                    ctr_char_is_next = true;
                }
            } else if ctr_char_is_next {
                match s[0] {
                    b'n' => writer.write_all(b"\n")?,
                    b't' => writer.write_all(b"\t")?,
                    b'\'' => writer.write_all(b"\'")?,
                    b'\"' => writer.write_all(b"\"")?,
                    _ => {}
                }
                writer.write_all(&s[1..])?;
                empty_slice = false;
                ctr_char_is_next = true;
            } else {
                writer.write_all(s)?;
                empty_slice = false;
                ctr_char_is_next = true;
            }
        }

        let consumed_bytes = buf.len();
        reader.consume(consumed_bytes);

        ctr_char_is_next = empty_slice;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_and_copy_simple_test() {
        let in_buf = b"abc\\ndef\\\\ghi".to_vec();
        let mut out_buf = Vec::<u8>::new();

        format_and_copy(&mut in_buf.as_slice(), &mut out_buf).expect("Error during log formatting");

        assert_eq!(out_buf, b"abc\ndef\\ghi");
    }
}
