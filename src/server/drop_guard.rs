use super::key_value_store::KeyValueStore;

/// A wrapper around a `Db` instance. This exists to allow orderly cleanup
/// of the `Db` by signalling the background purge task to shut down when
/// this struct is dropped.
#[derive(Debug)]
pub struct DropGuard {
    /// The `Db` instance that will be shut down when this `DbHolder` struct
    /// is dropped.
    pub kv_store: Box<dyn KeyValueStore>,
}

impl DropGuard {
    /// Create a new `DbHolder`, wrapping a `Db` instance. When this is dropped
    /// the `Db`'s purge task will be shut down.
    pub fn new(kv_store: Box<dyn KeyValueStore>) -> DropGuard {
        DropGuard { kv_store }
    }

    // // Get the shared database. Internally, this is an
    // // `Arc`, so a clone only increments the ref count.
    // pub fn kv_store(&'a self) -> Box<dyn KeyValueStore + 'a> {
    //     self.kv_store.clone()
    // }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        // Signal the 'Db' instance to shut down the task that purges expired keys
        self.kv_store.shutdown_purge_task();
    }
}
