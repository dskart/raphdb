use crate::cmd::Parse;
use crate::{Connection, Frame, KeyValueStore};

use bytes::Bytes;

#[derive(Debug)]
pub struct Set {
    key: String,
    value: Bytes,
}

impl Set {
    pub fn new(key: impl ToString, value: Bytes) -> Set {
        Set { key: key.to_string(), value }
    }

    pub fn parse_frames(parse: &mut Parse) -> crate::Result<Set> {
        let key = parse.next_string()?;
        let value = parse.next_bytes()?;

        Ok(Set { key, value })
    }

    pub async fn apply(self, kv: Box<dyn KeyValueStore>, dst: &mut Connection) -> crate::Result<()> {
        kv.set(self.key, self.value)?;

        let response = Frame::Simple("OK".to_string());
        dst.write_frame(&response).await?;

        Ok(())
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("set".as_bytes()));
        frame.push_bulk(Bytes::from(self.key.into_bytes()));
        frame.push_bulk(self.value);
        return frame;
    }
}
