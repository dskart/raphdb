use crate::{
    connection::{Connection, Frame, Parser},
    KeyValueStore,
};

use bytes::Bytes;

#[derive(Debug)]
pub struct Get {
    key: String,
}

impl Get {
    pub fn new(key: impl ToString) -> Get {
        Get { key: key.to_string() }
    }

    pub fn parse_frames(parser: &mut Parser) -> crate::Result<Get> {
        let key = parser.next_string()?;
        Ok(Get { key })
    }

    pub async fn apply(self, kv: Box<dyn KeyValueStore>, dst: &mut Connection) -> crate::Result<()> {
        let response = if let Some(value) = kv.get(&self.key)? {
            Frame::Bulk(value)
        } else {
            Frame::Null
        };

        dst.write_frame(&response).await?;

        Ok(())
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("get".as_bytes()));
        frame.push_bulk(Bytes::from(self.key.into_bytes()));
        return frame;
    }
}
