use crate::connection::{Connection, Frame};

#[derive(Debug)]
pub struct Unknown {
    command_name: String,
}

impl Unknown {
    pub fn new(key: impl ToString) -> Unknown {
        Unknown { command_name: key.to_string() }
    }

    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let response = Frame::Error(format!("ERR unknown command '{}'", self.command_name));

        dst.write_frame(&response).await?;
        Ok(())
    }
}
