mod get;
pub use get::Get;
mod set;
pub use set::Set;
mod unknown;
pub use unknown::Unknown;

use crate::{
    connection::{Connection, Frame, Parser},
    KeyValueStore,
};

#[derive(Debug)]
pub enum Command {
    Get(Get),
    Set(Set),
    Unknown(Unknown),
}

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let mut parser = Parser::new(frame)?;

        let command_name = parser.next_string()?.to_lowercase();

        let command = match &command_name[..] {
            "get" => Command::Get(Get::parse_frames(&mut parser)?),
            "set" => Command::Set(Set::parse_frames(&mut parser)?),
            _ => {
                // The command is not recognized and an Unknown command is
                // returned.
                //
                // `return` is called here to skip the `finish()` call below. As
                // the command is not recognized, there is most likely
                // unconsumed fields remaining in the `Parser` instance.
                return Ok(Command::Unknown(Unknown::new(command_name)));
            }
        };

        parser.finish()?;

        Ok(command)
    }

    pub async fn apply(self, kv: Box<dyn KeyValueStore>, dst: &mut Connection) -> crate::Result<()> {
        use Command::*;

        match self {
            Get(cmd) => cmd.apply(kv, dst).await,
            Set(cmd) => cmd.apply(kv, dst).await,
            Unknown(cmd) => cmd.apply(dst).await,
        }
    }
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_from_frame() {
        assert!(true)
    }
}
