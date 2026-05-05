use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "kivo", version, about = "A Redis-compatible in-memory key-value store")]
pub struct Config {
    /// IP address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to listen on
    #[arg(long, short, default_value_t = 6379)]
    pub port: u16,

    /// Authentication password. Leave empty to disable auth.
    #[arg(long, default_value = "secret")]
    pub password: String,

    /// Maximum number of keys before LRU eviction kicks in
    #[arg(long, default_value_t = 10_000)]
    pub max_keys: usize,

    /// Path to the persistence snapshot file
    #[arg(long, default_value = "data.kivo")]
    pub db_file: String,

    /// Interval in seconds between background snapshots
    #[arg(long, default_value_t = 10)]
    pub save_interval: u64,
}
