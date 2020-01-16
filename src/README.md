# What is riolog
`Riolog` is a command-line log viewer for a custom RIO logging format. Because it `less` viewer for viewing files and ANSI escape codes for coloring it is inteded to be used from GNU-compatible terminal emulator (on Linux - standard terminal, on Windows - Git Bash will be fine).

# Install
1. Install Rust by following instructions on https://rustup.rs/
2. Add `${HOME}/.cargo/bin` to PATH.
3. Go to riolog source directory and run:
```
$ cargo install --path .
```

# Usage
Typical usage of `riolog` is to open log file from a terminal:
```
$ riolog ls-2020-01-16_17-28-57.log
```
By default `riolog` opens in an interactive mode using `less` as an user interface. `Riolog` shows contents of the log file and additionally:
* replaces escaped control characters such as `\\n`, `\\t`, `\\"` with actual control codes
* colors log entries differently for different log levels (debug, info, warning, critical, fatal)
* filters log entries using multiple criteria (level, date/time)

`Riolog` can also run in non-interactive mode by saving its output to a file.
