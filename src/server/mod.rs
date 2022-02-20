mod drop_guard;
mod handler;
pub mod key_value_store;
mod listener;

use clap::{AppSettings, Arg};
use std::future::Future;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::{broadcast, mpsc, Semaphore};

use crate::server::{drop_guard::DropGuard, listener::Listener};
use key_value_store::*;

pub const CMD_NAME: &str = "start-server";

const BACKEND_ARG: &str = "backend";
pub fn cmd<'a, 'b>() -> clap::App<'a, 'b> {
    let backend_arg = Arg::with_name("backend")
        .value_name(BACKEND_ARG)
        .short("b")
        .long("backend")
        .takes_value(true)
        .required(true)
        .possible_values(&Backend::possible_names())
        .help("The KeyValueStore backend implementation.");

    clap::App::new("start-server")
        .about("starts a raphDB server")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(backend_arg)
}

pub const DEFAULT_PORT: &str = "6379";

pub async fn run(logger: slog::Logger, matches: &clap::ArgMatches<'_>) -> crate::Result<()> {
    let backend_name = matches.value_of(BACKEND_ARG).expect("backend arg is required");
    let backend = Backend::from_str(backend_name)?;

    info!(logger, "Starting raphDB server with KeyValueStore = {:?}", backend_name);

    let listener = TcpListener::bind(&format!("127.0.0.1:{}", DEFAULT_PORT)).await?;
    start_server(logger, listener, signal::ctrl_c(), backend).await;
    return Ok(());
}

const MAX_CONNECTIONS: usize = 250;

pub async fn start_server(logger: slog::Logger, listener: TcpListener, shutdown: impl Future, backend: Backend) {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

    let kv = get_kv_store(logger.clone(), backend).await.expect("kv store backend does not exist");

    let mut server = Listener {
        listener,
        db_holder: DropGuard::new(kv),
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        notify_shutdown,
        shutdown_complete_tx,
        shutdown_complete_rx,
    };

    tokio::select! {
        res = server.run(logger.clone()) => {
            // Errors encountered when handling individual connections do not
            // bubble up to this point.
            if let Err(err) = res {
                error!(logger, "err = {}, failed to accept", err);
            }
        }
        _ = shutdown => {
            info!(logger, "shutting down");
        }
    }

    // Extract the `shutdown_complete` receiver and transmitter
    // explicitly drop `shutdown_transmitter`. This is important, as the
    // `.await` below would otherwise never complete.
    let Listener {
        mut shutdown_complete_rx,
        shutdown_complete_tx,
        notify_shutdown,
        ..
    } = server;

    // When `notify_shutdown` is dropped, all tasks which have `subscribe`d will
    // receive the shutdown signal and can exit
    drop(notify_shutdown);
    // Drop final `Sender` so the `Receiver` below can complete
    drop(shutdown_complete_tx);

    // Wait for all active connections to finish processing. As the `Sender`
    // handle held by the listener has been dropped above, the only remaining
    // `Sender` instances are held by connection handler tasks. When those drop,
    // the `mpsc` channel will close and `recv()` will return `None`.
    let _ = shutdown_complete_rx.recv().await;
}
