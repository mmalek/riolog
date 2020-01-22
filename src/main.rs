mod cli;
mod error;
mod log_entry;
mod log_entry_reader;
mod log_entry_reader_mux;
mod result;

use crate::cli::{FilteringOptions, Options};
use crate::error::Error;
use crate::log_entry::{LogEntry, LogLevel};
use crate::log_entry_reader::LogEntryReader;
use crate::log_entry_reader_mux::LogEntryReaderMux;
use crate::result::Result;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use subslice::SubsliceExt;

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let opts = Options::read()?;

    let mut readers = Vec::new();

    for input_file in &opts.input_files {
        let input_file = File::open(&input_file)?;
        readers.push(BufReader::new(input_file));
    }

    if let Some(output_file) = &opts.output_file {
        let output_file = File::create(output_file)?;
        let writer = BufWriter::new(output_file);
        read_log(readers, writer, opts)?;
    } else {
        let mut less_command = Command::new("less");

        if opts.color_enabled {
            less_command.arg("-R");
        }

        if !opts.wrap {
            less_command.arg("-S");
        }

        let mut less_process = less_command
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let less_stdin = less_process
            .stdin
            .as_mut()
            .ok_or(Error::CannotUseLessStdin)?;

        let res = read_log(readers, less_stdin, opts);

        ignore_broken_pipe(res)?;

        less_process.wait()?;
    }

    Ok(())
}

fn read_log(mut readers: Vec<impl BufRead>, writer: impl Write, opts: Options) -> Result<()> {
    if opts.is_filtering_or_coloring() || readers.len() != 1 {
        let mut entry_iters: Vec<_> = readers
            .into_iter()
            .map(LogEntryReader::new)
            .map(|reader| filtering_iter(reader, opts.filtering_options.clone()))
            .collect();

        if entry_iters.len() == 1 {
            let entry_iter = entry_iters.pop().expect("No elements");
            copy_log_semantic(entry_iter, writer, opts.color_enabled, opts.input_files)
        } else {
            let reader = LogEntryReaderMux::new(entry_iters);
            copy_log_semantic(reader, writer, opts.color_enabled, opts.input_files)
        }
    } else {
        let reader = readers.pop().expect("No elements");
        copy_log_fast(reader, writer)
    }
}

fn ignore_broken_pipe(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error),
        Ok(()) => Ok(()),
    }
}

fn copy_log_fast(mut reader: impl BufRead, mut writer: impl Write) -> Result<()> {
    let mut ctr_char_is_next = false;

    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }

        let last_slice_is_empty = format_special_chars(buf, &mut writer, ctr_char_is_next, None)?;

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

fn filtering_iter(
    input: impl Iterator<Item = LogEntry>,
    FilteringOptions {
        time_from,
        time_to,
        min_level,
        contains,
    }: FilteringOptions,
) -> impl Iterator<Item = LogEntry> {
    input
        .skip_while(move |entry| {
            if let (Some(timestamp), Some(time_from)) = (entry.timestamp(), time_from) {
                timestamp < time_from
            } else {
                false
            }
        })
        .take_while(move |entry| {
            if let (Some(timestamp), Some(time_to)) = (entry.timestamp(), time_to) {
                timestamp < time_to
            } else {
                true
            }
        })
        .filter(move |entry| {
            if let (Some(min_level), Some(level)) = (min_level, entry.level()) {
                (level as i32) >= (min_level as i32)
            } else {
                true
            }
        })
        .filter(move |entry| {
            if let Some(contains) = &contains {
                entry.contents().find(contains.as_ref()).is_some()
            } else {
                true
            }
        })
}

fn copy_log_semantic(
    log_entries: impl Iterator<Item = LogEntry>,
    mut writer: impl Write,
    color_enabled: bool,
    input_files: Vec<PathBuf>,
) -> Result<()> {
    for entry in log_entries {
        let level = entry.level().filter(|_| color_enabled);

        if input_files.len() > 1 {
            if color_enabled {
                writer.write_all(b"\x1B[36m")?;
            }
            write!(writer, "{}: ", input_files[entry.source()].display())?;
            if color_enabled {
                writer.write_all(b"\x1B[0m")?;
            }
        }

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

        format_special_chars(entry.contents(), &mut writer, false, eol_seq)?;

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
