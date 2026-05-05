mod config;
mod state;
mod resp;
mod cmd;
mod session;

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use clap::Parser;
use tokio::net::TcpListener;
use kivo::KvStore;

use config::Config;
use state::ServerState;
use session::handle_client;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = Config::parse();
    let addr   = format!("{}:{}", config.host, config.port);

    println!("╔════════════════════════════════════════╗");
    println!("║             kivo  v0.2.0               ║");
    println!("║  Redis-compatible key-value server     ║");
    println!("╚════════════════════════════════════════╝");
    println!("  Listening on  : {addr}");
    println!("  Auth          : {}", if config.password.is_empty() { "disabled" } else { "enabled" });
    println!("  Max keys      : {}", config.max_keys);
    println!("  Snapshot file : {}", config.db_file);
    println!("  Save interval : {}s", config.save_interval);
    println!();

    let db_path = std::path::Path::new(&config.db_file).to_path_buf();
    let store   = KvStore::load(&db_path, config.max_keys).await?;

    let state = Arc::new(ServerState {
        store,
        total_commands: AtomicU64::new(0),
        total_connections: AtomicU64::new(0),
        config: config.clone(),
    });

    // --- Background task: active expiration + snapshot ---
    {
        let bg = Arc::clone(&state);
        let interval = config.save_interval;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

                let purged = bg.store.purge_expired();
                if purged > 0 {
                    println!("[kivo] Active expiry: removed {purged} expired keys");
                }

                if let Err(e) = bg.store.save(std::path::Path::new(&bg.config.db_file)).await {
                    eprintln!("[kivo] BGSAVE error: {e}");
                } else {
                    let n = bg.store.db_size();
                    println!("[kivo] BGSAVE OK — {n} keys persisted");
                }
            }
        });
    }

    // --- Accept connections ---
    let listener = TcpListener::bind(&addr).await?;
    println!("[kivo] Ready to accept connections.");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let s = Arc::clone(&state);
                tokio::spawn(async move { handle_client(stream, s).await });
            }
            Err(e) => eprintln!("[kivo] Accept error: {e}"),
        }
    }
}
