//! mini_kv – Redis-compatible in-memory key-value server

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use mini_kv::{KvStore, KvError};

// ---------------------------------------------------------------------------
// CLI configuration
// ---------------------------------------------------------------------------

#[derive(Parser, Debug, Clone)]
#[command(name = "mini_kv", version, about = "A Redis-compatible in-memory key-value store")]
struct Config {
    /// IP address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, short, default_value_t = 6379)]
    port: u16,

    /// Authentication password. Leave empty to disable auth.
    #[arg(long, default_value = "secret")]
    password: String,

    /// Maximum number of keys before LRU eviction kicks in
    #[arg(long, default_value_t = 10_000)]
    max_keys: usize,

    /// Path to the persistence snapshot file
    #[arg(long, default_value = "data.json")]
    db_file: String,

    /// Interval in seconds between background snapshots
    #[arg(long, default_value_t = 10)]
    save_interval: u64,
}

// ---------------------------------------------------------------------------
// Shared server state
// ---------------------------------------------------------------------------

struct ServerState {
    store: KvStore,
    config: Config,
    total_commands: AtomicU64,
    total_connections: AtomicU64,
}

// ---------------------------------------------------------------------------
// RESP helpers
// ---------------------------------------------------------------------------

fn resp_ok()                    -> &'static str  { "+OK\r\n" }
fn resp_pong()                  -> &'static str  { "+PONG\r\n" }
fn resp_queued()                -> &'static str  { "+QUEUED\r\n" }
fn resp_nil()                   -> &'static str  { "$-1\r\n" }
fn resp_empty_array()           -> &'static str  { "*0\r\n" }
fn resp_int(v: i64)             -> String        { format!(":{v}\r\n") }
fn resp_err(msg: &str)          -> String        { format!("-ERR {msg}\r\n") }
fn resp_simple(msg: &str)       -> String        { format!("+{msg}\r\n") }
fn resp_bulk(val: &str)         -> String        { format!("${}\r\n{val}\r\n", val.len()) }

fn resp_array(items: &[String]) -> String {
    let mut out = format!("*{}\r\n", items.len());
    for item in items { out.push_str(&resp_bulk(item)); }
    out
}

fn resp_opt_bulk(v: Option<String>) -> String {
    match v { Some(s) => resp_bulk(&s), None => resp_nil().to_string() }
}

fn resp_kv_result<T, F>(r: Result<T, KvError>, f: F) -> String
where F: FnOnce(T) -> String {
    match r { Ok(v) => f(v), Err(e) => e.to_resp() }
}

// ---------------------------------------------------------------------------
// RESP parser
// ---------------------------------------------------------------------------

async fn parse_command<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
) -> Result<Vec<String>, String> {
    let mut line = String::new();
    if reader.read_line(&mut line).await.map_err(|e| e.to_string())? == 0 {
        return Err("EOF".into());
    }
    let line = line.trim_end_matches(['\r', '\n']);
    if line.is_empty() { return Ok(vec![]); }

    if line.starts_with('*') {
        let n: usize = line[1..].parse().map_err(|_| "bad array length")?;
        let mut args = Vec::with_capacity(n);
        for _ in 0..n {
            let mut hdr = String::new();
            reader.read_line(&mut hdr).await.map_err(|e| e.to_string())?;
            let hdr = hdr.trim_end_matches(['\r', '\n']);
            if !hdr.starts_with('$') { return Err("expected bulk string".into()); }
            let len: usize = hdr[1..].parse().map_err(|_| "bad bulk len")?;
            let mut buf = vec![0u8; len + 2];
            reader.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
            args.push(String::from_utf8(buf[..len].to_vec()).map_err(|_| "invalid utf-8")?);
        }
        Ok(args)
    } else {
        Ok(line.split_whitespace().map(str::to_string).collect())
    }
}

// ---------------------------------------------------------------------------
// Client session
// ---------------------------------------------------------------------------

async fn handle_client(stream: TcpStream, state: Arc<ServerState>) {
    let peer = stream.peer_addr().ok();
    state.total_connections.fetch_add(1, Ordering::Relaxed);
    println!("[mini_kv] Client connected: {peer:?}");

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // auth disabled when password is empty
    let auth_required = !state.config.password.is_empty();
    let mut authenticated = !auth_required;
    let mut in_tx = false;
    let mut tx_queue: Vec<Vec<String>> = Vec::new();

    loop {
        let parts = match parse_command(&mut reader).await {
            Ok(p) => p,
            Err(e) => {
                if e != "EOF" { eprintln!("[mini_kv] Parse error from {peer:?}: {e}"); }
                println!("[mini_kv] Client disconnected: {peer:?}");
                return;
            }
        };
        if parts.is_empty() { continue; }

        state.total_commands.fetch_add(1, Ordering::Relaxed);
        let cmd = parts[0].to_uppercase();

        // --- Auth gate ---
        if !authenticated && cmd != "AUTH" && cmd != "QUIT" {
            writer.write_all(b"-NOAUTH Authentication required.\r\n").await.ok();
            continue;
        }

        // --- Transaction queueing ---
        if in_tx && !matches!(cmd.as_str(), "EXEC" | "DISCARD" | "MULTI" | "QUIT") {
            tx_queue.push(parts);
            writer.write_all(resp_queued().as_bytes()).await.ok();
            continue;
        }

        // --- Session-level commands ---
        let response: String = match cmd.as_str() {
            "AUTH" => {
                if parts.len() != 2 {
                    resp_err("wrong number of arguments for 'auth' command")
                } else if parts[1] == state.config.password {
                    authenticated = true;
                    resp_ok().to_string()
                } else {
                    "-WRONGPASS invalid username-password pair or user is disabled.\r\n".to_string()
                }
            }

            "QUIT" => {
                writer.write_all(resp_ok().as_bytes()).await.ok();
                println!("[mini_kv] Client quit: {peer:?}");
                return;
            }

            "MULTI" => {
                if in_tx {
                    resp_err("MULTI calls can not be nested")
                } else {
                    in_tx = true;
                    tx_queue.clear();
                    resp_ok().to_string()
                }
            }

            "DISCARD" => {
                if !in_tx {
                    resp_err("DISCARD without MULTI")
                } else {
                    in_tx = false;
                    tx_queue.clear();
                    resp_ok().to_string()
                }
            }

            "EXEC" => {
                if !in_tx {
                    resp_err("EXEC without MULTI")
                } else {
                    in_tx = false;
                    let queued = std::mem::take(&mut tx_queue);
                    let mut out = format!("*{}\r\n", queued.len());
                    for tx_parts in &queued {
                        let tx_cmd = tx_parts[0].to_uppercase();
                        out.push_str(&execute_command(&state.store, &tx_cmd, tx_parts));
                    }
                    out
                }
            }

            "PING" => {
                if parts.len() >= 2 {
                    resp_bulk(&parts[1..].join(" "))
                } else {
                    resp_pong().to_string()
                }
            }

            "INFO" => build_info(&state),

            // Select is a no-op stub (only db 0 supported)
            "SELECT" => {
                if parts.get(1).map(|s| s.as_str()) == Some("0") {
                    resp_ok().to_string()
                } else {
                    resp_err("DB index is out of range")
                }
            }

            // redis-cli compatibility stubs
            "COMMAND" => resp_empty_array().to_string(),
            "CONFIG" if parts.get(1).map(|s| s.to_uppercase()).as_deref() == Some("GET") => {
                resp_empty_array().to_string()
            }
            "OBJECT" => resp_bulk("embstr"),
            "CLIENT" => resp_ok().to_string(),
            "RESET"  => resp_simple("RESET"),

            _ => execute_command(&state.store, &cmd, &parts),
        };

        writer.write_all(response.as_bytes()).await.ok();
    }
}

// ---------------------------------------------------------------------------
// Command execution (also used inside EXEC)
// ---------------------------------------------------------------------------

fn execute_command(store: &KvStore, cmd: &str, parts: &[String]) -> String {
    macro_rules! args_eq {
        ($n:expr) => {
            if parts.len() != $n {
                return resp_err(&format!("wrong number of arguments for '{}' command", cmd.to_lowercase()));
            }
        };
    }
    macro_rules! args_ge {
        ($n:expr) => {
            if parts.len() < $n {
                return resp_err(&format!("wrong number of arguments for '{}' command", cmd.to_lowercase()));
            }
        };
    }

    match cmd {
        // --- Strings ---
        "SET" => {
            args_ge!(3);
            // Parse optional flags: EX seconds | PX ms | NX | XX | KEEPTTL
            let key = parts[1].clone();
            let val = parts[2].clone();
            let mut expire_ms: Option<i64> = None;
            let mut nx = false;
            let mut i = 3;
            while i < parts.len() {
                match parts[i].to_uppercase().as_str() {
                    "EX" => {
                        i += 1;
                        if i >= parts.len() { return resp_err("syntax error"); }
                        let secs: i64 = match parts[i].parse() {
                            Ok(v) => v, Err(_) => return resp_err("value is not an integer or out of range"),
                        };
                        expire_ms = Some(secs * 1000);
                    }
                    "PX" => {
                        i += 1;
                        if i >= parts.len() { return resp_err("syntax error"); }
                        let ms: i64 = match parts[i].parse() {
                            Ok(v) => v, Err(_) => return resp_err("value is not an integer or out of range"),
                        };
                        expire_ms = Some(ms);
                    }
                    "NX" => { nx = true; }
                    _ => { return resp_err("syntax error"); }
                }
                i += 1;
            }
            if nx {
                if store.setnx(key, val) { resp_ok().to_string() } else { resp_nil().to_string() }
            } else {
                resp_kv_result(store.set(key, val, expire_ms), |_| resp_ok().to_string())
            }
        }

        "SETEX" => {
            args_eq!(4);
            let secs: i64 = match parts[2].parse() {
                Ok(v) => v, Err(_) => return resp_err("value is not an integer or out of range"),
            };
            resp_kv_result(store.set(parts[1].clone(), parts[3].clone(), Some(secs * 1000)), |_| resp_ok().to_string())
        }

        "PSETEX" => {
            args_eq!(4);
            let ms: i64 = match parts[2].parse() {
                Ok(v) => v, Err(_) => return resp_err("value is not an integer or out of range"),
            };
            resp_kv_result(store.set(parts[1].clone(), parts[3].clone(), Some(ms)), |_| resp_ok().to_string())
        }

        "SETNX" => {
            args_eq!(3);
            let ok = store.setnx(parts[1].clone(), parts[2].clone());
            resp_int(ok as i64)
        }

        "GET" => {
            args_eq!(2);
            resp_kv_result(store.get(&parts[1]), resp_opt_bulk)
        }

        "GETSET" => {
            args_eq!(3);
            resp_kv_result(store.getset(parts[1].clone(), parts[2].clone()), resp_opt_bulk)
        }

        "MSET" => {
            if parts.len() < 3 || parts.len() % 2 == 0 {
                return resp_err("wrong number of arguments for 'mset' command");
            }
            let pairs = parts[1..].chunks(2).map(|c| (c[0].clone(), c[1].clone())).collect();
            store.mset(pairs);
            resp_ok().to_string()
        }

        "MGET" => {
            args_ge!(2);
            let keys = &parts[1..];
            let vals = store.mget(keys);
            let mut out = format!("*{}\r\n", vals.len());
            for v in vals { out.push_str(&resp_opt_bulk(v)); }
            out
        }

        "INCR"    => { args_eq!(2); resp_kv_result(store.incr_by(&parts[1], 1),  |v| resp_int(v)) }
        "DECR"    => { args_eq!(2); resp_kv_result(store.incr_by(&parts[1], -1), |v| resp_int(v)) }
        "INCRBY"  => {
            args_eq!(3);
            let d: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.incr_by(&parts[1], d), |v| resp_int(v))
        }
        "DECRBY"  => {
            args_eq!(3);
            let d: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.incr_by(&parts[1], -d), |v| resp_int(v))
        }

        "APPEND"  => {
            args_eq!(3);
            resp_kv_result(store.append(parts[1].clone(), parts[2].clone()), |len| resp_int(len as i64))
        }

        // --- Generic ---
        "DEL" | "UNLINK" => {
            args_ge!(2);
            resp_int(store.del(&parts[1..]))
        }

        "EXISTS"  => {
            args_eq!(2);
            resp_int(store.exists(&parts[1]) as i64)
        }

        "TYPE"    => {
            args_eq!(2);
            resp_simple(store.type_of(&parts[1]))
        }

        "RENAME"  => {
            args_eq!(3);
            resp_kv_result(store.rename(&parts[1], parts[2].clone()), |_| resp_ok().to_string())
        }

        "EXPIRE"  => {
            args_eq!(3);
            let secs: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.expire(&parts[1], secs * 1000), |ok| resp_int(ok as i64))
        }

        "PEXPIRE" => {
            args_eq!(3);
            let ms: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.expire(&parts[1], ms), |ok| resp_int(ok as i64))
        }

        "TTL"     => { args_eq!(2); resp_int(store.ttl(&parts[1])) }
        "PTTL"    => { args_eq!(2); resp_int(store.pttl(&parts[1])) }

        "PERSIST" => {
            args_eq!(2);
            resp_int(store.persist(&parts[1]) as i64)
        }

        "KEYS"    => {
            args_eq!(2);
            resp_array(&store.keys(&parts[1]))
        }

        "DBSIZE"  => resp_int(store.db_size() as i64),

        "FLUSHALL" | "FLUSHDB" => {
            store.flush_all();
            resp_ok().to_string()
        }

        // --- Hashes ---
        "HSET" => {
            // HSET key field value [field value ...]
            if parts.len() < 4 || (parts.len() - 2) % 2 != 0 {
                return resp_err("wrong number of arguments for 'hset' command");
            }
            let key = parts[1].clone();
            let mut added = 0i64;
            let pairs: Vec<_> = parts[2..].chunks(2)
                .map(|c| (c[0].clone(), c[1].clone()))
                .collect();
            for (f, v) in pairs {
                match store.hset(key.clone(), f, v) {
                    Ok(n) => added += n,
                    Err(e) => return e.to_resp(),
                }
            }
            resp_int(added)
        }

        "HGET"    => {
            args_eq!(3);
            resp_kv_result(store.hget(&parts[1], &parts[2]), resp_opt_bulk)
        }

        "HDEL"    => {
            args_eq!(3);
            resp_kv_result(store.hdel(&parts[1], &parts[2]), |ok| resp_int(ok as i64))
        }

        "HGETALL" => {
            args_eq!(2);
            resp_kv_result(store.hgetall(&parts[1]), |pairs| {
                let flat: Vec<String> = pairs.into_iter().flat_map(|(k, v)| [k, v]).collect();
                resp_array(&flat)
            })
        }

        "HKEYS"   => { args_eq!(2); resp_kv_result(store.hkeys(&parts[1]),  |v| resp_array(&v)) }
        "HVALS"   => { args_eq!(2); resp_kv_result(store.hvals(&parts[1]),  |v| resp_array(&v)) }
        "HLEN"    => { args_eq!(2); resp_kv_result(store.hlen(&parts[1]),   |v| resp_int(v as i64)) }

        "HEXISTS" => {
            args_eq!(3);
            resp_kv_result(store.hexists(&parts[1], &parts[2]), |ok| resp_int(ok as i64))
        }

        "HMGET"   => {
            args_ge!(3);
            resp_kv_result(store.hmget(&parts[1], &parts[2..]), |vals| {
                let mut out = format!("*{}\r\n", vals.len());
                for v in vals { out.push_str(&resp_opt_bulk(v)); }
                out
            })
        }

        "HMSET"   => {
            // Deprecated alias for HSET; same argument structure
            if parts.len() < 4 || (parts.len() - 2) % 2 != 0 {
                return resp_err("wrong number of arguments for 'hmset' command");
            }
            let key = parts[1].clone();
            for c in parts[2..].chunks(2) {
                if let Err(e) = store.hset(key.clone(), c[0].clone(), c[1].clone()) {
                    return e.to_resp();
                }
            }
            resp_ok().to_string()
        }

        // --- Lists ---
        "LPUSH"   => { args_eq!(3); resp_kv_result(store.lpush(parts[1].clone(), parts[2].clone()), |n| resp_int(n as i64)) }
        "RPUSH"   => { args_eq!(3); resp_kv_result(store.rpush(parts[1].clone(), parts[2].clone()), |n| resp_int(n as i64)) }
        "LPOP"    => { args_eq!(2); resp_kv_result(store.lpop(&parts[1]),  resp_opt_bulk) }
        "RPOP"    => { args_eq!(2); resp_kv_result(store.rpop(&parts[1]),  resp_opt_bulk) }
        "LLEN"    => { args_eq!(2); resp_kv_result(store.llen(&parts[1]),  |n| resp_int(n as i64)) }
        "LINDEX"  => {
            args_eq!(3);
            let idx: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.lindex(&parts[1], idx), resp_opt_bulk)
        }
        "LRANGE"  => {
            args_eq!(4);
            let start: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            let stop:  i64 = match parts[3].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.lrange(&parts[1], start, stop), |v| resp_array(&v))
        }

        _ => resp_err(&format!("unknown command `{}`, with args beginning with: {}", cmd.to_lowercase(),
            parts[1..].iter().take(3).map(|s| format!("`{s}`")).collect::<Vec<_>>().join(", "))),
    }
}

// ---------------------------------------------------------------------------
// INFO output
// ---------------------------------------------------------------------------

fn build_info(state: &ServerState) -> String {
    let uptime_secs = state.store.start_time.elapsed().as_secs();
    let total_keys  = state.store.db_size();
    let cmds        = state.total_commands.load(Ordering::Relaxed);
    let conns       = state.total_connections.load(Ordering::Relaxed);

    let body = format!(
"# Server\r\nredis_version:7.0.0-mini_kv\r\nredis_mode:standalone\r\nnos:rust\r\n\
arch_bits:64\r\nuptime_in_seconds:{uptime_secs}\r\n\
\r\n# Clients\r\ntotal_connections_received:{conns}\r\n\
\r\n# Stats\r\ntotal_commands_processed:{cmds}\r\n\
\r\n# Keyspace\r\ndb0:keys={total_keys},expires=0,avg_ttl=0\r\n"
    );
    resp_bulk(&body)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = Config::parse();
    let addr   = format!("{}:{}", config.host, config.port);

    println!("╔════════════════════════════════════════╗");
    println!("║           mini_kv  v0.2.0              ║");
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
                    println!("[mini_kv] Active expiry: removed {purged} expired keys");
                }

                if let Err(e) = bg.store.save(std::path::Path::new(&bg.config.db_file)).await {
                    eprintln!("[mini_kv] BGSAVE error: {e}");
                } else {
                    let n = bg.store.db_size();
                    println!("[mini_kv] BGSAVE OK — {n} keys persisted");
                }
            }
        });
    }

    // --- Accept connections ---
    let listener = TcpListener::bind(&addr).await?;
    println!("[mini_kv] Ready to accept connections.");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let s = Arc::clone(&state);
                tokio::spawn(async move { handle_client(stream, s).await });
            }
            Err(e) => eprintln!("[mini_kv] Accept error: {e}"),
        }
    }
}
