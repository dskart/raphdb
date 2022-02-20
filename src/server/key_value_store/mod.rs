pub mod mini_redis;
pub mod simple_store;

use bytes::Bytes;
use simple_error::bail;
use std::fmt::Debug;

use mini_redis::MiniRedis;
use simple_store::SimpleStore;

#[derive(Debug)]
pub enum Backend {
    SimpleStore,
    MiniRedis,
}

const MINI_REDIS: &str = "mini-redis";
const SIMPLE_STORE: &str = "simple-store";

impl Backend {
    pub fn from_str(backend_name: &str) -> crate::Result<Self> {
        match backend_name {
            MINI_REDIS => Ok(Backend::MiniRedis),
            SIMPLE_STORE => Ok(Backend::SimpleStore),
            _ => {
                bail!("Backend {:?} does not exist", backend_name)
            }
        }
    }

    pub fn possible_names() -> Vec<&'static str> {
        return vec![MINI_REDIS, SIMPLE_STORE];
    }
}

pub async fn get_kv_store(logger: slog::Logger, backend: Backend) -> crate::Result<Box<dyn KeyValueStore>> {
    match backend {
        Backend::MiniRedis => Ok(Box::new(MiniRedis::new())),
        Backend::SimpleStore => Ok(Box::new(SimpleStore::new(logger).await?)),
    }
}

pub trait KeyValueStore: Debug + KeyValueStoreClone + Send + Sync {
    fn get(&self, key: &str) -> crate::Result<Option<Bytes>>;
    fn set(&self, key: String, value: Bytes) -> crate::Result<()>;
    fn shutdown_purge_task(&self);
}

pub trait KeyValueStoreClone {
    fn clone_box(&self) -> Box<dyn KeyValueStore>;
}

impl<T> KeyValueStoreClone for T
where
    T: 'static + KeyValueStore + Clone,
{
    fn clone_box(&self) -> Box<dyn KeyValueStore> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn KeyValueStore> {
    fn clone(&self) -> Box<dyn KeyValueStore> {
        self.clone_box()
    }
}
