mod cli;
mod error;
mod filtering;
mod formatting;
mod log_entry;
mod log_entry_reader;
mod log_entry_reader_mux;
mod result;

use crate::cli::Options;
use crate::error::Error;
use crate::filtering::filtering_iter;
use crate::formatting::format_special_chars;
use crate::log_entry::{LogEntry, LogLevel};
use crate::log_entry_reader::LogEntryReader;
use crate::log_entry_reader_mux::LogEntryReaderMux;
use crate::result::Result;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

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
        let input_file =
            File::open(&input_file).map_err(|e| Error::CannotOpenFile(input_file.clone(), e))?;
        readers.push(BufReader::new(input_file));
    }

    if let Some(output_file) = &opts.output_file {
        let output_file = File::create(output_file)
            .map_err(|e| Error::CannotCreateFile(output_file.clone(), e))?;
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
            write_log(entry_iter, writer, opts.color_enabled, &opts.input_files)
        } else {
            let reader = LogEntryReaderMux::new(entry_iters);
            write_log(reader, writer, opts.color_enabled, &opts.input_files)
        }
    } else {
        let reader = readers.pop().expect("No elements");
        write_log_fast(reader, writer)
    }
}

fn ignore_broken_pipe(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

fn write_log_fast(mut reader: impl BufRead, mut writer: impl Write) -> Result<()> {
    let mut ctr_char_is_next = false;

    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }

        let last_slice_is_empty =
            format_special_chars(buf, &mut writer, ctr_char_is_next, b"", b"")?;

        let consumed_bytes = buf.len();
        reader.consume(consumed_bytes);

        ctr_char_is_next = last_slice_is_empty;
    }

    Ok(())
}

const CODE_CYAN: &[u8; 5] = b"\x1B[36m";
const CODE_GRAY: &[u8; 5] = b"\x1B[37m";
const CODE_RED: &[u8; 5] = b"\x1B[31m";
const CODE_RED_BRIGHT: &[u8; 5] = b"\x1B[91m";
const CODE_WHITE: &[u8; 5] = b"\x1B[97m";
const CODE_YELLOW: &[u8; 5] = b"\x1B[33m";
const CODE_NORMAL: &[u8; 4] = b"\x1B[0m";

fn write_log(
    log_entries: impl Iterator<Item = LogEntry>,
    mut writer: impl Write,
    color_enabled: bool,
    input_files: &[PathBuf],
) -> Result<()> {
    for entry in log_entries {
        if input_files.len() > 1 {
            if color_enabled {
                writer.write_all(CODE_CYAN)?;
            }
            write!(writer, "{}: ", input_files[entry.source()].display())?;
            if color_enabled {
                writer.write_all(CODE_NORMAL)?;
            }
        }

        let level = if color_enabled { entry.level() } else { None };

        let color_code: &[u8] = match level {
            Some(LogLevel::Debug) => CODE_GRAY,
            Some(LogLevel::Info) => CODE_WHITE,
            Some(LogLevel::Warning) => CODE_YELLOW,
            Some(LogLevel::Critical) => CODE_RED,
            Some(LogLevel::Fatal) => CODE_RED_BRIGHT,
            None => b"",
        };

        writer.write_all(color_code)?;

        format_special_chars(
            entry.contents(),
            &mut writer,
            false,
            CODE_NORMAL,
            color_code,
        )?;

        if color_enabled {
            writer.write_all(CODE_NORMAL)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    const LOREM_IPSUM: &[u8] = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed \
do eiusmod tempor incididunt ut labore et dolore magna aliqua. In eu mi bibendum neque egestas \
congue quisque egestas diam. Amet tellus cras adipiscing enim. Feugiat vivamus at augue eget \
arcu dictum varius duis at. Integer enim neque volutpat ac tincidunt. Lobortis elementum nibh \
tellus molestie. A iaculis at erat pellentesque. Porttitor rhoncus dolor purus non enim praesent \
elementum facilisis. Sed blandit libero volutpat sed cras. Consectetur lorem donec massa sapien \
faucibus et molestie.";

    const ELEMENTUM_EU: &[u8] = b"Elementum eu facilisis sed odio morbi quis commodo odio \
aenean. Quis eleifend quam adipiscing vitae. A iaculis at erat pellentesque adipiscing. Pulvinar \
pellentesque habitant morbi tristique senectus et netus et malesuada. Commodo elit at imperdiet \
dui. Volutpat est velit egestas dui id. Dictum sit amet justo donec enim diam vulputate ut. \
Blandit libero volutpat sed cras ornare arcu dui vivamus arcu. At urna condimentum mattis \
pellentesque id nibh tortor id aliquet. Quam pellentesque nec nam aliquam sem et tortor.";

    const DT_FORMAT: &str = "%F %T.%3f";

    fn header(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> Vec<u8> {
        let buf = format!(
            "-info:<22954> {} UTC [Category]: ",
            NaiveDate::from_ymd(year, month, day)
                .and_hms(hour, min, sec)
                .format(DT_FORMAT)
        );
        buf.into_bytes()
    }

    #[test]
    fn write_log_fast_simple_test() -> Result<()> {
        let in_buf = b"abc\\ndef\\\\ghi".to_vec();
        let mut out_buf = Vec::<u8>::new();

        write_log_fast(&mut in_buf.as_slice(), &mut out_buf)?;

        assert_eq!(out_buf, b"abc\ndef\\ghi");

        Ok(())
    }

    #[test]
    fn write_log_fast_single_entry() -> Result<()> {
        let mut in_buf = Vec::new();
        in_buf.append(&mut header(2020, 01, 13, 20, 42, 00));
        in_buf.append(&mut LOREM_IPSUM.to_vec());
        in_buf.append(&mut b"\n\n".to_vec());

        let mut out_buf = Vec::<u8>::new();

        write_log_fast(&mut in_buf.as_slice(), &mut out_buf)?;

        assert_eq!(out_buf, in_buf);

        Ok(())
    }

    #[test]
    fn write_log_fast_two_entries() -> Result<()> {
        let mut in_buf = Vec::new();

        in_buf.append(&mut header(2020, 01, 13, 20, 42, 00));
        in_buf.append(&mut LOREM_IPSUM.to_vec());
        in_buf.append(&mut b"\n\n".to_vec());

        in_buf.append(&mut header(2020, 01, 13, 20, 43, 00));
        in_buf.append(&mut ELEMENTUM_EU.to_vec());
        in_buf.append(&mut b"\n\n".to_vec());

        let mut out_buf = Vec::<u8>::new();

        write_log_fast(&mut in_buf.as_slice(), &mut out_buf)?;

        assert_eq!(out_buf, in_buf);

        Ok(())
    }

    #[test]
    fn write_log_single_entry_uncolored() -> Result<()> {
        let mut contents = Vec::new();
        contents.append(&mut header(2020, 01, 13, 20, 42, 00));
        contents.append(&mut LOREM_IPSUM.to_vec());
        contents.append(&mut b"\n\n".to_vec());

        let entry = LogEntry::new(contents.clone());
        let mut entries = vec![entry];
        let input_files = [PathBuf::from("logname")];

        let mut out_buf = Vec::<u8>::new();

        write_log(entries.drain(..), &mut out_buf, false, &input_files)?;

        let mut in_buf = Vec::new();
        in_buf.append(&mut contents);
        assert_eq!(out_buf, in_buf);

        Ok(())
    }

    #[test]
    fn write_log_two_entries_uncolored() -> Result<()> {
        let mut contents1 = Vec::new();
        contents1.append(&mut header(2020, 01, 13, 20, 42, 00));
        contents1.append(&mut LOREM_IPSUM.to_vec());
        contents1.append(&mut b"\n\n".to_vec());
        let entry1 = LogEntry::new(contents1.clone());

        let mut contents2 = Vec::new();
        contents2.append(&mut header(2020, 01, 13, 20, 43, 00));
        contents2.append(&mut ELEMENTUM_EU.to_vec());
        contents2.append(&mut b"\n\n".to_vec());
        let entry2 = LogEntry::new(contents2.clone());

        let mut entries = vec![entry1, entry2];
        let input_files = [PathBuf::from("logname")];

        let mut out_buf = Vec::<u8>::new();

        write_log(entries.drain(..), &mut out_buf, false, &input_files)?;

        let mut in_buf = Vec::new();
        in_buf.append(&mut contents1);
        in_buf.append(&mut contents2);
        assert_eq!(out_buf, in_buf);

        Ok(())
    }
}
