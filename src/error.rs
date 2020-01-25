use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    CannotOpenFile(PathBuf, io::Error),
    CannotCreateFile(PathBuf, io::Error),
    CannotUseLessStdin,
    InvalidCliOptionValue(&'static str),
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(error) => write!(f, "IO error: {}", error),
            Error::CannotOpenFile(file, error) => {
                write!(f, "Cannot open file {}: {}", file.display(), error)
            }
            Error::CannotCreateFile(file, error) => {
                write!(f, "Cannot create file {}: {}", file.display(), error)
            }
            Error::CannotUseLessStdin => write!(f, "Cannot open stdin stream for 'less' process"),
            Error::InvalidCliOptionValue(opt) => write!(
                f,
                "Invalid value provided for command line option '{}'",
                opt
            ),
        }
    }
}
