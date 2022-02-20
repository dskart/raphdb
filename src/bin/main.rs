#[macro_use]
extern crate slog;

// use raphdb::{key_value_store::KeyValueStoreClient, simple_store_client::SimpleStoreClient};

use clap::{AppSettings, Arg};
use simple_error::bail;
use slog::Drain;
use std::os::unix::io::AsRawFd;

use raphdb::{client, server, Result};

pub async fn exec(logger: slog::Logger, matches: &clap::ArgMatches<'_>) -> Result<()> {
    match matches.subcommand() {
        (server::CMD_NAME, Some(matches)) => server::run(logger, matches).await,
        (client::CMD_NAME, Some(matches)) => client::run(logger, matches).await,
        ("", None) => bail!("no subcommand was used"),
        _ => unreachable!("match arms should cover all the possible cases"),
    }
}

pub async fn set_up_logger_and_exec() -> i32 {
    let matches = clap::App::new("raphdb")
        .about("raphdb")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(Arg::with_name("debug").short("d").long("debug").help("makes the logs more verbose"))
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(server::cmd())
        .subcommand(client::cmd())
        .get_matches();

    let stderr = std::io::stderr();
    let drain: Box<dyn Drain<Ok = (), Err = std::io::Error> + Send> = match termios::Termios::from_fd(stderr.as_raw_fd() as _) {
        Ok(_) => {
            let decorator = slog_term::TermDecorator::new().build();
            Box::new(slog_term::FullFormat::new(decorator).build())
        }
        Err(_) => Box::new(slog_json::Json::default(stderr)),
    };

    let drain = slog_async::Async::new(drain.fuse())
        .build()
        .filter_level(if matches.is_present("debug") { slog::Level::Debug } else { slog::Level::Info })
        .fuse();

    let logger = slog::Logger::root(drain, o!());

    if let Err(e) = exec(logger.clone(), &matches).await {
        error!(logger, "{}", e);
        1
    } else {
        0
    }
}

#[tokio::main]
async fn main() {
    std::process::exit(set_up_logger_and_exec().await)
}
