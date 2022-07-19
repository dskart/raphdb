use tokio::sync::broadcast;

#[derive(Debug)]
pub struct Shutdown {
    shutdown: bool,
    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver`.
    pub fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown { shutdown: false, notify }
    }

    /// Returns `true` if the shutdown signal has been received.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub async fn recv(&mut self) {
        if self.shutdown {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.notify.recv().await;

        self.shutdown = true;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_shutdown() {
        let (notify_shutdown, _) = broadcast::channel(1);
        let mut shutdown = Shutdown::new(notify_shutdown.subscribe());
        assert_eq!(shutdown.is_shutdown(), false);

        drop(notify_shutdown);
        shutdown.recv().await;

        assert_eq!(shutdown.is_shutdown(), true);
    }
}
