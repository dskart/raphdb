use crate::Result;

use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{BufRead, Seek, Write};
use std::sync::{Arc, Mutex, RwLock};

use bytes::{BufMut, Bytes, BytesMut};
use simple_error::bail;
use tokio::io::AsyncBufReadExt;

use super::KeyValueStore;

#[derive(Debug, Clone)]
pub struct SimpleStore {
    logger: slog::Logger,
    shared: Arc<Shared>,
}

#[derive(Debug)]
struct Shared {
    /// The shared state is guarded by a mutex. This is a `std::sync::Mutex` and
    /// not a Tokio mutex. This is because there are no asynchronous operations
    /// being performed while holding the mutex. Additionally, the critical
    /// sections are very small.
    ///
    /// A Tokio mutex is mostly intended to be used when locks need to be held
    /// across `.await` yield points. All other cases are **usually** best
    /// served by a std mutex. If the critical section does not include any
    /// async operations but is long (CPU intensive or performing blocking
    /// operations), then the entire operation, including waiting for the mutex,
    /// is considered a "blocking" operation and `tokio::task::spawn_blocking`
    /// should be used.
    // state: Mutex<State>,
    state: RwLock<State>,
    write_mutex: Mutex<()>,
}

#[derive(Debug)]
struct State {
    /// Hash Index of the keys -> location in the file
    index: HashMap<String, usize>,

    /// True when the  instance is shutting down. This happens when all `SimpleSTore`
    /// values drop. Setting this to `true` signals to the background task to
    /// exit.
    shutdown: bool,
}

const LOG_FILE: &str = "log.raphdb";

impl SimpleStore {
    pub async fn new(logger: slog::Logger) -> Result<SimpleStore> {
        let index = SimpleStore::init(logger.clone()).await?;

        let shared = Arc::new(Shared {
            state: RwLock::new(State { index, shutdown: false }),
            write_mutex: Mutex::new(()),
        });

        return Ok(SimpleStore { logger, shared });
    }

    pub async fn init(logger: slog::Logger) -> Result<HashMap<String, usize>> {
        let attr = tokio::fs::metadata(LOG_FILE).await;
        match attr {
            Ok(_) => {
                info!(logger, "Found log file, recovering indexes...");
                let index = SimpleStore::recover().await?;
                info!(logger, "Recovered {:?} indexes.", index.len());
                return Ok(index);
            }
            Err(_) => {
                info!(logger, "No log file found, creating new log file...");
                tokio::fs::File::create(LOG_FILE).await?;
                info!(logger, "Log file created!");
                return Ok(HashMap::new());
            }
        }
    }

    pub async fn recover() -> Result<HashMap<String, usize>> {
        let file = tokio::fs::OpenOptions::new().read(true).open(LOG_FILE).await?;
        let reader = tokio::io::BufReader::new(file);
        let mut lines = reader.lines();

        let mut index = HashMap::new();
        let mut byte_offset: usize = 0;
        while let Some(line) = lines.next_line().await? {
            let key_value: Vec<&str> = line.split(",").collect();
            if key_value.len() < 2 {
                bail!("log file data is corrupted at byte {:?}", byte_offset);
            }

            let key = key_value[0];
            index.insert(key.to_string(), byte_offset);

            // +1 is for the /n byte
            byte_offset += line.len() + 1;
        }

        return Ok(index);
    }
}

impl KeyValueStore for SimpleStore {
    fn get(&self, key: &str) -> crate::Result<Option<Bytes>> {
        let offset: usize;
        {
            let state = self.shared.state.read().unwrap();
            match state.index.get(key).clone() {
                Some(byte_offset) => offset = byte_offset.clone(),
                None => return Ok(None),
            }
        }

        let mut data = String::new();
        {
            let mut file = std::fs::OpenOptions::new().read(true).open(LOG_FILE)?;
            file.seek(std::io::SeekFrom::Start(offset.try_into().unwrap()))?;
            let mut reader = std::io::BufReader::new(file);
            reader.read_line(&mut data)?;
        }

        let mut buf = BytesMut::new();
        let key_value: Vec<&str> = data.split(",").collect();
        if key_value.len() < 2 {
            bail!("Index key = {:?} log data is corrupted", key);
        } else if key_value[0] != key {
            bail!("log data key = {:?} does not match index key = {:?}", key_value[0], key);
        }

        buf.put(key_value[1..].join("").as_bytes());

        debug!(self.logger, "Get: {:?} | {:?}", key, buf.clone());
        return Ok(Some(buf.into()));
    }

    fn set(&self, key: String, value: Bytes) -> crate::Result<()> {
        let mut buf = BytesMut::new();
        buf.put(key.as_bytes());
        buf.put_u8(b',');
        buf.put(value.clone());
        buf.put_u8(b'\n');

        let len: u64;
        {
            let _m = self.shared.write_mutex.lock().unwrap();
            let mut file = std::fs::OpenOptions::new().append(true).open(LOG_FILE)?;
            len = file.metadata()?.len();
            file.write_all(&buf[..])?;
            file.sync_all()?;
        }

        {
            let mut state = self.shared.state.write().unwrap();
            state.index.insert(key.to_string(), len.try_into().unwrap());
        }

        debug!(self.logger, "Set: {:?} | {:?}", key, value);
        return Ok(());
    }

    fn shutdown_purge_task(&self) {}
}
