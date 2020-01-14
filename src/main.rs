mod error;
mod log_entry;
mod log_entry_reader;
mod result;

use crate::error::Error;
use crate::log_entry::{LogEntry, LogLevel};
use crate::log_entry_reader::LogEntryReader;
use crate::result::Result;
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
            let reader = LogEntryReader::new(reader);
            copy_log_colored(reader, &mut writer)?;
        } else {
            copy_log_fast(&mut reader, &mut writer)?;
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
            let reader = LogEntryReader::new(reader);
            copy_log_colored(reader, &mut less_stdin)
        } else {
            copy_log_fast(&mut reader, &mut less_stdin)
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

fn copy_log_fast(reader: &mut impl BufRead, writer: &mut impl Write) -> Result<()> {
    let mut ctr_char_is_next = false;

    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }

        let last_slice_is_empty = format_special_chars(buf, writer, ctr_char_is_next, None)?;

        let consumed_bytes = buf.len();
        reader.consume(consumed_bytes);

        ctr_char_is_next = last_slice_is_empty;
    }

    Ok(())
}

fn format_special_chars(
    buf: &[u8],
    writer: &mut impl Write,
    mut ctr_char_is_next: bool,
    eol_seq: Option<&[u8]>,
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
                b'n' => writer.write_all(eol_seq.unwrap_or(b"\n"))?,
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

fn copy_log_colored(
    log_entries: impl Iterator<Item = LogEntry>,
    writer: &mut impl Write,
) -> Result<()> {
    for entry in log_entries {
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
        let eol_seq = level.map(|level| match level {
            LogLevel::Debug => &b"\x1B[0m\n\x1B[37m"[..],
            LogLevel::Info => &b"\x1B[0m\n\x1B[97m"[..],
            LogLevel::Warning => &b"\x1B[0m\n\x1B[33m"[..],
            LogLevel::Critical => &b"\x1B[0m\n\x1B[31m"[..],
            LogLevel::Fatal => &b"\x1B[0m\n\x1B[91m"[..],
        });
        format_special_chars(&entry.contents, writer, false, eol_seq)?;
        writer.write_all(b"\x1B[0m")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_log_fast_simple_test() {
        let in_buf = b"abc\\ndef\\\\ghi".to_vec();
        let mut out_buf = Vec::<u8>::new();

        copy_log_fast(&mut in_buf.as_slice(), &mut out_buf).expect("Error during log formatting");

        assert_eq!(out_buf, b"abc\ndef\\ghi");
    }
}
