mod cli;
mod error;
mod log_entry;
mod log_entry_reader;
mod result;

use crate::cli::Options;
use crate::error::Error;
use crate::log_entry::{LogEntry, LogLevel};
use crate::log_entry_reader::LogEntryReader;
use crate::result::Result;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

fn main() -> Result<()> {
    let opts = Options::read()?;

    let input_file = File::open(&opts.input_file)?;
    let mut reader = BufReader::new(input_file);

    if let Some(output_file) = &opts.output_file {
        let output_file = File::create(output_file)?;
        let mut writer = BufWriter::new(output_file);

        if opts.is_filtering_or_coloring() {
            let reader = LogEntryReader::new(reader);
            copy_log_semantic(reader, &mut writer, opts)?;
        } else {
            copy_log_fast(&mut reader, &mut writer)?;
        }
    } else {
        let mut less_command = Command::new("less");

        if opts.color_enabled {
            less_command.arg("-R");
        }

        let mut less_process = less_command
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut less_stdin = less_process
            .stdin
            .as_mut()
            .ok_or(Error::CannotUseLessStdin)?;

        let res = if opts.is_filtering_or_coloring() {
            let reader = LogEntryReader::new(reader);
            copy_log_semantic(reader, &mut less_stdin, opts)
        } else {
            copy_log_fast(&mut reader, &mut less_stdin)
        };

        ignore_broken_pipe(res)?;

        less_process.wait()?;
    }

    Ok(())
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

fn copy_log_semantic(
    log_entries: impl Iterator<Item = LogEntry>,
    writer: &mut impl Write,
    Options {
        color_enabled,
        mut time_from,
        time_to,
        min_level,
        ..
    }: Options,
) -> Result<()> {
    for entry in log_entries {
        if time_from.is_some() || time_to.is_some() {
            if let Some(timestamp) = entry.timestamp() {
                if let Some(time_f) = &time_from {
                    if timestamp < *time_f {
                        continue;
                    } else {
                        // Since entries are sorted we no longer
                        // need to check this filter
                        time_from = None;
                    }
                }
                if let Some(time_to) = &time_to {
                    if timestamp >= *time_to {
                        break;
                    }
                }
            }
        }

        let level = if color_enabled || min_level.is_some() {
            let level = entry.level();
            if let Some(min_level) = &min_level {
                if let Some(level) = &level {
                    if (*level as i32) < (*min_level as i32) {
                        continue;
                    }
                }
            }
            level.filter(|_| color_enabled)
        } else {
            None
        };

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

        if color_enabled {
            writer.write_all(b"\x1B[0m")?;
        }
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
