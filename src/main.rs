mod cli;
mod direction;
mod eol;
mod error;
mod filtering;
mod formatting;
mod log_entry;
mod log_entry_reader;
mod log_entry_reader_mux;
mod result;
mod rev_reader;

use crate::cli::Options;
use crate::direction::Direction;
use crate::error::Error;
use crate::filtering::filtering_iter;
use crate::formatting::format_special_chars;
use crate::log_entry::{LogEntry, LogLevel};
use crate::log_entry_reader::{LogEntryReader, LogEntryRevReader};
use crate::log_entry_reader_mux::LogEntryReaderMux;
use crate::result::Result;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use streaming_iterator::StreamingIterator;

const IO_BUF_SIZE: usize = 1024 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let opts = Options::read()?;

    if let Some(output_file) = &opts.output_file {
        let writer = File::create(output_file)
            .map(|w| BufWriter::with_capacity(IO_BUF_SIZE, w))
            .map_err(|e| Error::CannotCreateFile(output_file.clone(), e))?;
        read_log(writer, opts)
    } else if opts.pager {
        let mut less_command = Command::new("less");
        less_command.arg("--quit-if-one-screen");

        if opts.color_enabled {
            less_command.arg("--RAW-CONTROL-CHARS");
        }

        if !opts.wrap {
            less_command.arg("--chop-long-lines");
        }

        let mut less_process = less_command
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let writer = less_process
            .stdin
            .as_mut()
            .map(|w| BufWriter::with_capacity(IO_BUF_SIZE, w))
            .ok_or(Error::CannotUseLessStdin)?;

        let res = read_log(writer, opts);

        ignore_broken_pipe(res)?;

        less_process.wait()?;

        Ok(())
    } else {
        let stdout = std::io::stdout();
        let writer = BufWriter::with_capacity(IO_BUF_SIZE, stdout.lock());
        ignore_broken_pipe(read_log(writer, opts))
    }
}

fn read_log(writer: impl Write, opts: Options) -> Result<()> {
    if opts.input_files.is_empty()
        || (opts.input_files.len() == 1 && opts.input_files[0] == Path::new("-"))
    {
        let stdin = io::stdin();
        if opts.is_filtering_or_coloring() {
            let reader = LogEntryReader::new(stdin.lock(), eol::EOL);
            let reader = filtering_iter(reader, opts.filtering_options.clone(), Direction::Forward);
            write_log(
                reader,
                writer,
                opts.color_enabled,
                opts.formatting_enabled,
                &opts.input_files,
            )
        } else {
            write_log_fast(stdin.lock(), writer, opts.formatting_enabled)
        }
    } else if opts.is_filtering_or_coloring() || opts.input_files.len() > 1 || opts.reverse {
        let readers: Result<Vec<_>> = opts
            .input_files
            .iter()
            .map(|f| File::open(&f).map_err(|e| Error::CannotOpenFile(f.clone(), e)))
            .map(|f| f.map(|f| BufReader::with_capacity(IO_BUF_SIZE, f)))
            .collect();

        let readers = readers?;

        if opts.reverse {
            let mut entry_iters: Vec<_> = readers
                .into_iter()
                .enumerate()
                .map(|(i, r)| {
                    let reader = LogEntryRevReader::with_capacity(r, eol::EOL, IO_BUF_SIZE)?;
                    Ok(filtering_iter(
                        reader.with_source(i),
                        opts.filtering_options.clone(),
                        Direction::Reverse,
                    ))
                })
                .collect::<Result<_>>()?;

            if entry_iters.len() == 1 {
                let entry_iter = entry_iters.pop().expect("No elements");
                write_log(
                    entry_iter,
                    writer,
                    opts.color_enabled,
                    opts.formatting_enabled,
                    &opts.input_files,
                )
            } else {
                let reader = LogEntryReaderMux::new(entry_iters, Direction::Reverse);
                write_log(
                    reader,
                    writer,
                    opts.color_enabled,
                    opts.formatting_enabled,
                    &opts.input_files,
                )
            }
        } else {
            let mut entry_iters: Vec<_> = readers
                .into_iter()
                .enumerate()
                .map(|(i, r)| LogEntryReader::new(r, eol::EOL).with_source(i))
                .map(|reader| {
                    filtering_iter(reader, opts.filtering_options.clone(), Direction::Forward)
                })
                .collect();

            if entry_iters.len() == 1 {
                let entry_iter = entry_iters.pop().expect("No elements");
                write_log(
                    entry_iter,
                    writer,
                    opts.color_enabled,
                    opts.formatting_enabled,
                    &opts.input_files,
                )
            } else {
                let reader = LogEntryReaderMux::new(entry_iters, Direction::Forward);
                write_log(
                    reader,
                    writer,
                    opts.color_enabled,
                    opts.formatting_enabled,
                    &opts.input_files,
                )
            }
        }
    } else {
        let file = opts.input_files.first().expect("No elements");
        let file = File::open(&file).map_err(|e| Error::CannotOpenFile(file.clone(), e))?;
        let reader = BufReader::with_capacity(IO_BUF_SIZE, file);
        write_log_fast(reader, writer, opts.formatting_enabled)
    }
}

fn ignore_broken_pipe(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

fn write_log_fast(
    mut reader: impl BufRead,
    mut writer: impl Write,
    formatting: bool,
) -> Result<()> {
    let mut ctr_char_is_next = false;

    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }

        if formatting {
            ctr_char_is_next =
                format_special_chars(buf, &mut writer, ctr_char_is_next, eol::EOL, b"")?;
        } else {
            writer.write_all(buf)?;
        }

        let consumed_bytes = buf.len();
        reader.consume(consumed_bytes);
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
    mut log_entries: impl StreamingIterator<Item = LogEntry>,
    mut writer: impl Write,
    color_enabled: bool,
    formatting: bool,
    input_files: &[PathBuf],
) -> Result<()> {
    let code_normal_eol = [CODE_NORMAL, eol::EOL].concat();

    while let Some(entry) = log_entries.next() {
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

        let eol: &[u8] = if color_enabled {
            &code_normal_eol
        } else {
            eol::EOL
        };

        let color_code: &[u8] = match level {
            Some(LogLevel::Debug) => CODE_GRAY,
            Some(LogLevel::Info) => CODE_WHITE,
            Some(LogLevel::Warning) => CODE_YELLOW,
            Some(LogLevel::Critical) => CODE_RED,
            Some(LogLevel::Fatal) => CODE_RED_BRIGHT,
            None => b"",
        };

        writer.write_all(color_code)?;

        if formatting {
            format_special_chars(entry.contents(), &mut writer, false, eol, color_code)?;
        } else {
            writer.write_all(entry.contents())?;
        }

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
    use eol;

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

        write_log_fast(&mut in_buf.as_slice(), &mut out_buf, true)?;

        let mut pattern = b"abc".to_vec();
        pattern.append(&mut eol::EOL.to_vec());
        pattern.append(&mut b"def\\ghi".to_vec());

        assert_eq!(out_buf, pattern);

        Ok(())
    }

    #[test]
    fn write_log_fast_single_entry() -> Result<()> {
        let mut in_buf = Vec::new();
        in_buf.append(&mut header(2020, 01, 13, 20, 42, 00));
        in_buf.append(&mut LOREM_IPSUM.to_vec());
        in_buf.append(&mut b"\n\n".to_vec());

        let mut out_buf = Vec::<u8>::new();

        write_log_fast(&mut in_buf.as_slice(), &mut out_buf, true)?;

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

        write_log_fast(&mut in_buf.as_slice(), &mut out_buf, true)?;

        assert_eq!(out_buf, in_buf);

        Ok(())
    }

    #[test]
    fn write_log_single_entry_uncolored() -> Result<()> {
        let mut contents = Vec::new();
        contents.append(&mut header(2020, 01, 13, 20, 42, 00));
        contents.append(&mut LOREM_IPSUM.to_vec());
        contents.append(&mut b"\n\n".to_vec());

        let entry = LogEntry::from_contents(contents.clone());
        let entries = vec![entry];
        let input_files = [PathBuf::from("logname")];

        let mut out_buf = Vec::<u8>::new();

        write_log(
            streaming_iterator::convert(entries),
            &mut out_buf,
            false,
            true,
            &input_files,
        )?;

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
        let entry1 = LogEntry::from_contents(contents1.clone());

        let mut contents2 = Vec::new();
        contents2.append(&mut header(2020, 01, 13, 20, 43, 00));
        contents2.append(&mut ELEMENTUM_EU.to_vec());
        contents2.append(&mut b"\n\n".to_vec());
        let entry2 = LogEntry::from_contents(contents2.clone());

        let entries = vec![entry1, entry2];
        let input_files = [PathBuf::from("logname")];

        let mut out_buf = Vec::<u8>::new();

        write_log(
            streaming_iterator::convert(entries),
            &mut out_buf,
            false,
            true,
            &input_files,
        )?;

        let mut in_buf = Vec::new();
        in_buf.append(&mut contents1);
        in_buf.append(&mut contents2);
        assert_eq!(out_buf, in_buf);

        Ok(())
    }
}
