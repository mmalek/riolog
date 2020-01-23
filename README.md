# What is riolog
`Riolog` is a command-line log viewer for a custom logging format. Because it uses `less` viewer for viewing files and ANSI escape codes for coloring it is inteded to be used from GNU-compatible terminal emulator (on Linux - standard terminal, on Windows - Git Bash will be fine).

# Install using binary
Go into https://github.com/mmalek/riolog/releases and download the latest release. `riolog` is a single self-contained binary. The only runtime dependency is `less` command which is available in a *nix environment. For best experience put `riolog` binary into directory listed on `PATH` environment variable.
Alternatively you can build `riolog` from source which is fairly easy - see next section.

# Install from source
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

# Features
* highlighting by priority level (debug - gray, info - white, warning - yellow, critical - red, fatal - bright red)
* pretty-printing of log entries by replacing escaped control characters such as `\\n`, `\\t`, `\\"` with actual control codes
* merging multiple log files into one view chronologically
* filtering using multiple criteria (level, date/time, contents)
* interactive scrolling using `less` as an user interface
* non-interactive mode: saving to a file

