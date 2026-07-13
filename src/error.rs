use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    InvalidChannel(u8),
    InvalidConfiguration(String),
    InvalidInput(String),
    Io(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidChannel(channel) => {
                write!(
                    formatter,
                    "BLE channel {channel} is outside the valid range 0..=39"
                )
            }
            Self::InvalidConfiguration(message) => formatter.write_str(message),
            Self::InvalidInput(message) => formatter.write_str(message),
            Self::Io(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
