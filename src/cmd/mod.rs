mod get;
pub use get::Get;
mod set;
pub use set::Set;
mod unknown;
pub use unknown::Unknown;

use crate::{Connection, Frame, KeyValueStore, Parse};

#[derive(Debug)]
pub enum Command {
    Get(Get),
    Set(Set),
    Unknown(Unknown),
}

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let mut parse = Parse::new(frame)?;

        let command_name = parse.next_string()?.to_lowercase();

        let command = match &command_name[..] {
            "get" => Command::Get(Get::parse_frames(&mut parse)?),
            "set" => Command::Set(Set::parse_frames(&mut parse)?),
            _ => {
                // The command is not recognized and an Unknown command is
                // returned.
                //
                // `return` is called here to skip the `finish()` call below. As
                // the command is not recognized, there is most likely
                // unconsumed fields remaining in the `Parse` instance.
                return Ok(Command::Unknown(Unknown::new(command_name)));
            }
        };

        parse.finish()?;

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
