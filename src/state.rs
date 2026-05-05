use std::sync::atomic::AtomicU64;
use kivo::KvStore;
use crate::config::Config;

pub struct ServerState {
    pub store: KvStore,
    pub config: Config,
    pub total_commands: AtomicU64,
    pub total_connections: AtomicU64,
}
