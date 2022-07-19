use crate::connection::FrameError;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::convert::TryInto;
use std::fmt;
use std::io::Cursor;

#[derive(Clone, Debug)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}

impl Frame {
    /// Returns an empty array
    pub fn array() -> Frame {
        Frame::Array(vec![])
    }

    /// Push a "bulk" frame into the array. `self` must be an Array frame.
    ///
    /// # Panics
    ///
    /// panics if `self` is not an array
    pub fn push_bulk(&mut self, bytes: Bytes) {
        match self {
            Frame::Array(vec) => {
                vec.push(Frame::Bulk(bytes));
            }
            _ => panic!("not an array frame"),
        }
    }

    /// Push an "integer" frame into the array. `self` must be an Array frame.
    ///
    /// # Panics
    ///
    /// panics if `self` is not an Frame::Array
    #[allow(dead_code)]
    pub fn push_int(&mut self, value: u64) {
        match self {
            Frame::Array(vec) => {
                vec.push(Frame::Integer(value));
            }
            _ => panic!("not an array frame"),
        }
    }

    /// Creates bytes from the corresponding Frame
    ///
    /// # Panics
    ///
    /// Panics if self is a Frame::Array as async does not allow recursion
    pub fn create_bytes(&self) -> std::io::Result<BytesMut> {
        let mut buffer = BytesMut::new();
        match self {
            Frame::Simple(val) => {
                buffer.put_u8(b'+');
                buffer.put(val.as_bytes());
                buffer.put(&b"\r\n"[..]);
            }
            Frame::Error(val) => {
                buffer.put_u8(b'-');
                buffer.put(val.as_bytes());
                buffer.put(&b"\r\n"[..]);
            }
            Frame::Integer(val) => {
                buffer.put_u8(b':');
                buffer.put(val.to_string().as_bytes());
                buffer.put(&b"\r\n"[..]);
            }
            Frame::Null => {
                buffer.put(&b"$-1\r\n"[..]);
            }
            Frame::Bulk(val) => {
                let len = val.len();
                buffer.put_u8(b'$');
                buffer.put(len.to_string().as_bytes());
                buffer.put(&b"\r\n"[..]);
                buffer.put(val.clone());
                buffer.put(&b"\r\n"[..]);
            }
            // Encoding an `Array` from within a value cannot be done using a
            // recursive strategy. In general, async fns do not support
            // recursion.
            Frame::Array(_val) => unreachable!(),
        };

        return Ok(buffer);
    }

    /// Checks if an entire frame can be decoded from `src`.
    /// Will return an Incomplete Error if the src does not have enough bytes to
    /// parse a whole frame.
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        match get_u8(src)? {
            b'+' => {
                get_line(src)?;
                Ok(())
            }
            b'-' => {
                get_line(src)?;
                Ok(())
            }
            b':' => {
                let _ = get_decimal(src)?;
                Ok(())
            }
            b'$' => {
                if b'-' == peek_u8(src)? {
                    skip(src, 4) // Skip '-1\r\n'
                } else {
                    let len: usize = get_decimal(src)?.try_into()?;
                    skip(src, len + 2) // skip that number of bytes + 2 (\r\n).
                }
            }
            b'*' => {
                let len = get_decimal(src)?;
                for _ in 0..len {
                    Frame::check(src)?;
                }
                Ok(())
            }
            actual => Err(format!("protocol error; invalid frame type byte `{}`", actual).into()),
        }
    }

    /// Parser `src` into a Frame. This method should be called after `Frame::check(src)`.
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, FrameError> {
        match get_u8(src)? {
            b'+' => {
                let line = get_line(src)?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(Frame::Simple(string))
            }
            b'-' => {
                let line = get_line(src)?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(Frame::Error(string))
            }
            b':' => {
                let len = get_decimal(src)?;
                Ok(Frame::Integer(len))
            }
            b'$' => {
                if b'-' == peek_u8(src)? {
                    let line = get_line(src)?;

                    if line != b"-1" {
                        return Err("protocol error; invalid frame format".into());
                    }

                    Ok(Frame::Null)
                } else {
                    let len = get_decimal(src)?.try_into()?;
                    let n = len + 2;
                    if src.remaining() < n {
                        return Err(FrameError::Incomplete);
                    }
                    let data = Bytes::copy_from_slice(&src.chunk()[..len]);
                    skip(src, n)?; // skip that number of bytes + 2 (\r\n).
                    Ok(Frame::Bulk(data))
                }
            }
            b'*' => {
                let len = get_decimal(src)?.try_into()?;
                let mut out = Vec::with_capacity(len);

                for _ in 0..len {
                    out.push(Frame::parse(src)?);
                }

                Ok(Frame::Array(out))
            }
            _ => unimplemented!(),
        }
    }

    pub fn to_error(&self) -> crate::Error {
        format!("unexpected frame: {}", self).into()
    }
}

impl PartialEq<&str> for Frame {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Frame::Simple(s) => s.eq(other),
            Frame::Bulk(s) => s.eq(other),
            _ => false,
        }
    }
}

impl PartialEq<Frame> for Frame {
    fn eq(&self, other: &Frame) -> bool {
        match (self, other) {
            (Self::Simple(l0), Self::Simple(r0)) => l0 == r0,
            (Self::Error(l0), Self::Error(r0)) => l0 == r0,
            (Self::Integer(l0), Self::Integer(r0)) => l0 == r0,
            (Self::Bulk(l0), Self::Bulk(r0)) => l0 == r0,
            (Self::Array(l0), Self::Array(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use std::str;

        match self {
            Frame::Simple(response) => response.fmt(fmt),
            Frame::Error(msg) => write!(fmt, "error: {}", msg),
            Frame::Integer(num) => num.fmt(fmt),
            Frame::Bulk(msg) => match str::from_utf8(msg) {
                Ok(string) => string.fmt(fmt),
                Err(_) => write!(fmt, "{:?}", msg),
            },
            Frame::Null => "(nil)".fmt(fmt),
            Frame::Array(parts) => {
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        write!(fmt, " ")?;
                        part.fmt(fmt)?;
                    }
                }
                Ok(())
            }
        }
    }
}

fn peek_u8(src: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !src.has_remaining() {
        return Err(FrameError::Incomplete);
    }

    Ok(src.chunk()[0])
}

fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !src.has_remaining() {
        return Err(FrameError::Incomplete);
    }

    Ok(src.get_u8())
}

fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), FrameError> {
    if src.remaining() < n {
        return Err(FrameError::Incomplete);
    }

    src.advance(n);
    Ok(())
}

/// Read a new-line terminated decimal
fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<u64, FrameError> {
    use atoi::atoi;

    let line = get_line(src)?;

    atoi::<u64>(line).ok_or_else(|| "protocol error; invalid frame format".into())
}

/// Find a line
fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], FrameError> {
    // Scan the bytes directly
    let start = src.position() as usize;
    // Scan to the second to last byte
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            // We found a line, update the position to be *after* the \n
            src.set_position((i + 2) as u64);

            // Return the line
            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(FrameError::Incomplete)
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::BytesMut;

    struct CommonFrames {
        pub frames_and_expected_bytes: Vec<(Frame, BytesMut)>,
    }

    impl Default for CommonFrames {
        fn default() -> Self {
            return CommonFrames {
                frames_and_expected_bytes: vec![
                    (Frame::Simple("foo".to_string()), BytesMut::from("+foo\r\n")),
                    (Frame::Error("foo".to_string()), BytesMut::from("-foo\r\n")),
                    (Frame::Integer(10), BytesMut::from(":10\r\n")),
                    (Frame::Null, BytesMut::from("$-1\r\n")),
                    (Frame::Bulk(Bytes::from("foo")), BytesMut::from("$3\r\nfoo\r\n")),
                ],
            };
        }
    }

    impl CommonFrames {
        fn frames(&self) -> Vec<Frame> {
            return self.frames_and_expected_bytes.iter().map(|(f, _)| f.clone()).collect();
        }
    }

    #[tokio::test]
    async fn test_array() {
        let frame = Frame::array();
        let expected: Vec<Frame> = vec![];
        assert!(matches!(frame, Frame::Array(vec) if vec == expected));
    }

    #[tokio::test]
    async fn test_push_bulk() {
        let mut frame = Frame::array();
        let bytes = Bytes::from("Hello world");
        frame.push_bulk(bytes.clone());

        let expected = vec![Frame::Bulk(bytes)];
        assert!(matches!(frame, Frame::Array(vec) if vec == expected));
    }

    #[tokio::test]
    async fn test_push_int() {
        let mut frame = Frame::array();
        let integer: u64 = 10;
        frame.push_int(integer);

        let expected = vec![Frame::Integer(integer)];
        assert!(matches!(frame, Frame::Array(vec) if vec == expected));
    }

    #[tokio::test]
    async fn test_create_bytes() {
        for (frame, expected_bytes) in CommonFrames::default().frames_and_expected_bytes.iter() {
            let bytes = frame.create_bytes();
            assert!(bytes.is_ok());
            assert_eq!(bytes.unwrap(), expected_bytes)
        }
    }

    #[tokio::test]
    async fn test_check() {
        for (frame, _) in CommonFrames::default().frames_and_expected_bytes {
            let bytes = frame.create_bytes().unwrap();
            let mut buf = Cursor::new(&bytes[..]);
            assert!(Frame::check(&mut buf).is_ok())
        }

        // Check for array frame
        let frames = CommonFrames::default().frames();
        let mut bytes = BytesMut::from("*");
        bytes.put(frames.len().to_string().as_bytes());
        bytes.put(&b"\r\n"[..]);
        for frame in frames.iter() {
            bytes.put(frame.create_bytes().unwrap())
        }
        let mut buf = Cursor::new(&bytes[..]);
        assert!(Frame::check(&mut buf).is_ok());

        // Check for array frame err
        let bytes = BytesMut::from("err");
        let mut buf = Cursor::new(&bytes[..]);
        assert!(Frame::check(&mut buf).is_err())
    }

    #[tokio::test]
    async fn test_parse() {
        for (frame, _) in CommonFrames::default().frames_and_expected_bytes {
            let bytes = frame.create_bytes().unwrap();
            let mut buf = Cursor::new(&bytes[..]);

            let parsed_frame = Frame::parse(&mut buf);
            assert!(parsed_frame.is_ok());
            assert_eq!(parsed_frame.unwrap(), frame)
        }

        // Check for array frame
        let frames = CommonFrames::default().frames();
        let mut bytes = BytesMut::from("*");
        bytes.put(frames.len().to_string().as_bytes());
        bytes.put(&b"\r\n"[..]);
        for frame in frames.iter() {
            bytes.put(frame.create_bytes().unwrap())
        }
        let mut buf = Cursor::new(&bytes[..]);
        let parsed_frame = Frame::parse(&mut buf);
        assert!(parsed_frame.is_ok());
        assert_eq!(parsed_frame.unwrap(), Frame::Array(frames));
    }
}
