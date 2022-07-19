use crate::connection::{Frame, ParserError};

use bytes::Bytes;
use std::{str, vec};

#[derive(Debug)]
pub struct Parser {
    parts: vec::IntoIter<Frame>,
}

impl Parser {
    /// Create a new `Parser` to parser the contents of `frame`.
    ///
    /// Returns `Err` if `frame` is not an array frame.
    pub fn new(frame: Frame) -> Result<Parser, ParserError> {
        let array = match frame {
            Frame::Array(array) => array,
            frame => return Err(format!("protocol error; expected array, got {:?}", frame).into()),
        };

        Ok(Parser { parts: array.into_iter() })
    }

    /// Return the next entry. Array frames are arrays of frames, so the next
    /// entry is a frame.
    fn next(&mut self) -> Result<Frame, ParserError> {
        self.parts.next().ok_or(ParserError::EndOfStream)
    }

    /// Return the next entry as a string.
    ///
    /// If the next entry cannot be represented as a String, then an error is returned.
    pub fn next_string(&mut self) -> Result<String, ParserError> {
        match self.next()? {
            // Both `Simple` and `Bulk` representation may be strings. Strings
            // are parsed to UTF-8.
            //
            // While errors are stored as strings, they are considered separate
            // types.
            Frame::Simple(s) => Ok(s),
            Frame::Bulk(data) => str::from_utf8(&data[..])
                .map(|s| s.to_string())
                .map_err(|_| "protocol error; invalid string".into()),
            frame => Err(format!("protocol error; expected simple frame or bulk frame, got {:?}", frame).into()),
        }
    }

    /// Return the next entry as raw bytes.
    ///
    /// If the next entry cannot be represented as raw bytes, an error is
    /// returned.
    pub fn next_bytes(&mut self) -> Result<Bytes, ParserError> {
        match self.next()? {
            // Both `Simple` and `Bulk` representation may be raw bytes.
            //
            // Although errors are stored as strings and could be represented as
            // raw bytes, they are considered separate types.
            Frame::Simple(s) => Ok(Bytes::from(s.into_bytes())),
            Frame::Bulk(data) => Ok(data),
            frame => Err(format!("protocol error; expected simple frame or bulk frame, got {:?}", frame).into()),
        }
    }

    /// Return the next entry as an integer.
    ///
    /// This includes `Simple`, `Bulk`, and `Integer` frame types. `Simple` and
    /// `Bulk` frame types are parsed.
    ///
    /// If the next entry cannot be represented as an integer, then an error is
    /// returned.
    #[allow(dead_code)]
    pub fn next_int(&mut self) -> Result<u64, ParserError> {
        use atoi::atoi;

        const MSG: &str = "protocol error; invalid number";

        match self.next()? {
            // An integer frame type is already stored as an integer.
            Frame::Integer(v) => Ok(v),
            // Simple and bulk frames must be parsed as integers. If the parsing
            // fails, an error is returned.
            Frame::Simple(data) => atoi::<u64>(data.as_bytes()).ok_or_else(|| MSG.into()),
            Frame::Bulk(data) => atoi::<u64>(&data).ok_or_else(|| MSG.into()),
            frame => Err(format!("protocol error; expected int frame but got {:?}", frame).into()),
        }
    }

    /// Ensure there are no more entries in the array
    pub fn finish(&mut self) -> Result<(), ParserError> {
        if self.parts.next().is_none() {
            Ok(())
        } else {
            Err("protocol error; expected end of frame, but there was more".into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn create_array_frame() -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("Hello world"));
        frame.push_int(1);
        return frame;
    }

    #[tokio::test]
    async fn test_new() {
        let frame = Frame::Simple("foo".to_string());
        let parser = Parser::new(frame);
        assert!(parser.is_err());

        let frame = create_array_frame();
        let parser = Parser::new(frame);
        assert!(parser.is_ok());
    }

    #[tokio::test]
    async fn test_next() {
        let mut frame_array = Frame::array();
        frame_array.push_bulk(Bytes::from("Hello world"));
        frame_array.push_int(1);

        let mut parser = Parser::new(frame_array.clone()).unwrap();

        if let Frame::Array(frames) = frame_array {
            for frame in frames.into_iter() {
                let next_frame = parser.next();
                assert!(next_frame.is_ok());
                assert_eq!(next_frame.unwrap(), frame);
            }
            assert!(parser.finish().is_ok());
            assert!(parser.next().is_err());
        } else {
            unreachable!()
        }
    }
}
