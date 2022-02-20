pub mod client;

use clap::{AppSettings, Arg, SubCommand};

pub const CMD_NAME: &str = "start-client";

const CMD_SET_NAME: &str = "set";
const CMD_GET_NAME: &str = "get";
const KEY_ARG: &str = "key";
const VALUE_ARG: &str = "value";

pub fn cmd<'a, 'b>() -> clap::App<'a, 'b> {
    let key_arg = Arg::with_name("key")
        .short("k")
        .long("key")
        .takes_value(true)
        .required(true)
        .value_name(KEY_ARG)
        .help("The value's key.");

    let value_arg = Arg::with_name("value")
        .long("value")
        .takes_value(true)
        .required(true)
        .value_name(VALUE_ARG)
        .help("The keys's value key.");

    clap::App::new(CMD_NAME)
        .about("starts a raphDB client")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name(CMD_SET_NAME)
                .about("Sets a key/value pair.")
                .arg(key_arg.clone())
                .arg(value_arg),
        )
        .subcommand(SubCommand::with_name(CMD_GET_NAME).about("Gets the value from a key.").arg(key_arg))
}

pub async fn run(logger: slog::Logger, matches: &clap::ArgMatches<'_>) -> crate::Result<()> {
    info!(logger, "Starting raphDB client");
    let client = client::connect("127.0.0.1:6379").await?;

    match matches.subcommand() {
        (CMD_SET_NAME, Some(m)) => {
            let key = m.value_of(KEY_ARG).expect("key arg is required");
            let value = m.value_of(VALUE_ARG).expect("value arg is required").to_string();
            set(logger, client, key, value).await?
        }
        (CMD_GET_NAME, Some(m)) => {
            let key = m.value_of(KEY_ARG).expect("key arg is required");
            get(logger, client, key).await?;
        }
        _ => unreachable!("match arms should cover all the possible cases"),
    }

    Ok(())
}

pub async fn set(logger: slog::Logger, mut client: client::Client, key: &str, value: String) -> crate::Result<()> {
    info!(logger, "Setting key: {:?} | value: {:?}", key, value);
    client.set(key, value.into()).await?;
    return Ok(());
}

pub async fn get(logger: slog::Logger, mut client: client::Client, key: &str) -> crate::Result<()> {
    info!(logger, "Getting value from key: {:?}", key);
    let result = client.get(key).await?;
    info!(logger, "KEY = {:?} | VALUE = {:?}", key, result);
    return Ok(());
}
