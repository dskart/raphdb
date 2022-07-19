use std::fmt;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum FrameError {
    Incomplete,
    Other(crate::Error),
}

impl From<String> for FrameError {
    fn from(src: String) -> FrameError {
        FrameError::Other(src.into())
    }
}

impl From<&str> for FrameError {
    fn from(src: &str) -> FrameError {
        src.to_string().into()
    }
}

impl From<FromUtf8Error> for FrameError {
    fn from(_src: FromUtf8Error) -> FrameError {
        "protocol error; invalid frame format".into()
    }
}

impl From<TryFromIntError> for FrameError {
    fn from(_src: TryFromIntError) -> FrameError {
        "protocol error; invalid frame format".into()
    }
}

impl std::error::Error for FrameError {}

impl fmt::Display for FrameError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrameError::Incomplete => "stream ended early".fmt(fmt),
            FrameError::Other(err) => err.fmt(fmt),
        }
    }
}

#[derive(Debug)]
pub enum ParserError {
    EndOfStream,
    Other(crate::Error),
}

impl From<String> for ParserError {
    fn from(src: String) -> ParserError {
        ParserError::Other(src.into())
    }
}

impl From<&str> for ParserError {
    fn from(src: &str) -> ParserError {
        src.to_string().into()
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::EndOfStream => "protocol error; unexpected end of stream".fmt(f),
            ParserError::Other(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ParserError {}
