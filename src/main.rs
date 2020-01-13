mod error;
mod result;

use crate::error::Error;
use crate::result::Result;
use chrono::NaiveDateTime;
use clap::{App, Arg, ArgMatches};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

const ARG_FILE_NAME: &str = "file-name";
const ARG_COLOR: &str = "color";
const ARG_OUTPUT: &str = "output";
const ARG_VALUES_TRUE: [&str; 3] = ["yes", "true", "on"];
const ARG_VALUES_FALSE: [&str; 3] = ["no", "false", "off"];

fn main() -> Result<()> {
    let matches = App::new("riolog")
        .version("0.1")
        .about("Hi-Fi log parser")
        .arg(
            Arg::with_name(ARG_FILE_NAME)
                .help("path to a log file")
                .index(1)
                .required(true),
        )
        .arg(Arg::with_name(ARG_COLOR)
            .long(ARG_COLOR)
            .short("c")
            .value_name("yes/no")
            .help("turn off colorized output. Default: 'yes' for interactive mode, 'no' for file output mode (-o)"))
        .arg(Arg::with_name(ARG_OUTPUT)
            .long(ARG_OUTPUT)
            .short("o")
            .value_name("FILE")
            .help("write the log to the output file"))
        .get_matches();

    let output_file = matches.value_of_os(ARG_OUTPUT);

    let color_enabled =
        parse_bool_arg(&matches, ARG_COLOR)?.unwrap_or_else(|| output_file.is_none());

    let input_file_name = matches
        .value_of("file-name")
        .expect("missing \"file-name\" parameter");

    let input_file = File::open(input_file_name)?;
    let mut reader = BufReader::new(input_file);

    if let Some(output_file) = output_file {
        let output_file = File::create(output_file)?;
        let mut writer = BufWriter::new(output_file);

        if color_enabled {
            parse_log_entries(&mut reader, &mut writer)?;
        } else {
            filter_escape_sequences(&mut reader, &mut writer)?;
        }
    } else {
        let mut less_command = Command::new("less");

        if color_enabled {
            less_command.arg("-r");
        }

        let mut less_process = less_command
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut less_stdin = less_process
            .stdin
            .as_mut()
            .ok_or(Error::CannotUseLessStdin)?;

        let res = if color_enabled {
            parse_log_entries(&mut reader, &mut less_stdin)
        } else {
            filter_escape_sequences(&mut reader, &mut less_stdin)
        };

        ignore_broken_pipe(res)?;

        less_process.wait()?;
    }

    Ok(())
}

fn parse_bool_arg(matches: &ArgMatches, arg_name: &str) -> Result<Option<bool>> {
    if let Some(value) = matches.value_of(ARG_COLOR) {
        let value = value.to_lowercase();
        if ARG_VALUES_TRUE.iter().any(|&v| v == value) {
            Ok(Some(true))
        } else if ARG_VALUES_FALSE.iter().any(|&v| v == value) {
            Ok(Some(false))
        } else {
            Err(Error::InvalidCliOptionValue(arg_name.to_owned()))
        }
    } else {
        Ok(None)
    }
}

fn ignore_broken_pipe(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error),
        Ok(()) => Ok(()),
    }
}

fn filter_escape_sequences(reader: &mut impl BufRead, writer: &mut impl Write) -> Result<()> {
    let mut ctr_char_is_next = false;

    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }

        let last_slice_is_empty = filter_escseq_and_write(buf, writer, ctr_char_is_next)?;

        let consumed_bytes = buf.len();
        reader.consume(consumed_bytes);

        ctr_char_is_next = last_slice_is_empty;
    }

    Ok(())
}

fn filter_escseq_and_write(
    buf: &[u8],
    writer: &mut impl Write,
    mut ctr_char_is_next: bool,
) -> Result<bool> {
    let mut last_slice_is_empty = false;

    for s in buf.split(|&c| c == b'\\') {
        if s.is_empty() {
            if ctr_char_is_next {
                writer.write_all(b"\\")?;
                last_slice_is_empty = true;
                ctr_char_is_next = false;
            } else {
                last_slice_is_empty = false;
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
            last_slice_is_empty = false;
            ctr_char_is_next = true;
        } else {
            writer.write_all(s)?;
            last_slice_is_empty = false;
            ctr_char_is_next = true;
        }
    }

    Ok(last_slice_is_empty)
}

enum LogLevel {
    Debug,
    Info,
    Warning,
    Critical,
    Fatal,
}

struct LogEntry {
    contents: Vec<u8>,
}

impl LogEntry {
    fn level(&self) -> Option<LogLevel> {
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

    fn timestamp(&self) -> Option<NaiveDateTime> {
        self.contents
            .iter()
            .position(|&c| c == b'>')
            .map(|pos| pos + 2)
            .and_then(|pos| self.contents.get(pos..pos + 23))
            .and_then(|input| parse_timestamp(input).ok())
    }
}

fn parse_timestamp(input: &[u8]) -> Result<NaiveDateTime> {
    let input = String::from_utf8_lossy(input);
    NaiveDateTime::parse_from_str(&input, "%F %T.%3f")
        .map_err(|_| Error::CannotParseTimestamp(input.to_string()))
}

fn parse_log_entries(reader: &mut impl BufRead, writer: &mut impl Write) -> Result<()> {
    while let Some(entry) = parse_log_entry(reader)? {
        let level = entry.level();
        if let Some(level) = &level {
            match level {
                LogLevel::Debug => writer.write_all(b"\x1B[37m")?,
                LogLevel::Info => writer.write_all(b"\x1B[97m")?,
                LogLevel::Warning => writer.write_all(b"\x1B[33m")?,
                LogLevel::Critical => writer.write_all(b"\x1B[31m")?,
                LogLevel::Fatal => writer.write_all(b"\x1B[91m")?,
            }
        }
        filter_escseq_and_write(&entry.contents, writer, false)?;
        writer.write_all(b"\x1B[0m")?;
    }

    Ok(())
}

fn parse_log_entry(reader: &mut impl BufRead) -> Result<Option<LogEntry>> {
    let mut contents = Vec::new();
    loop {
        let bytes_read = reader.read_until(b'\n', &mut contents)?;
        // read until an empty line (might be '\n' or '\r\n')
        if bytes_read <= 2 {
            break;
        }
    }

    if contents.is_empty() {
        Ok(None)
    } else {
        Ok(Some(LogEntry { contents }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn filter_escape_sequences_simple_test() {
        let in_buf = b"abc\\ndef\\\\ghi".to_vec();
        let mut out_buf = Vec::<u8>::new();

        filter_escape_sequences(&mut in_buf.as_slice(), &mut out_buf)
            .expect("Error during log formatting");

        assert_eq!(out_buf, b"abc\ndef\\ghi");
    }

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
