use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use mini_kv::KvStore;

const DB_FILE: &str = "data.json";

fn handle_client(mut stream: TcpStream, store: Arc<Mutex<KvStore>>) {
    let peer = stream.peer_addr().ok();
    println!("Client connected: {:?}", peer);

    let mut reader = BufReader::new(stream.try_clone().unwrap());

    loop {
        let mut line = String::new();
        let bytes_read = match reader.read_line(&mut line) {
            Ok(0) => {
                println!("Client disconnected: {:?}", peer);
                return;
            }
            Ok(n) => n,
            Err(e) => {
                eprintln!("Read error from {:?}: {e}", peer);
                return;
            }
        };

        if bytes_read == 0 {
            println!("Client disconnected: {:?}", peer);
            return;
        }

        let line = line.trim_end_matches(&['\r', '\n'][..]);
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_uppercase();

        match cmd.as_str() {
            // ---------- SET commands (SADD, SREM, SMEMBERS, SISMEMBER) ----------
            "SADD" => {
                if parts.len() < 3 {
                    writeln!(stream, "ERR usage: SADD key member [member ...]").ok();
                    continue;
                }
                let key = parts[1];
                let members: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();
                let mut store_guard = store.lock().unwrap();
                match store_guard.sadd(key, members) {
                    Ok(added) => {
                        if let Err(e) = store_guard.save_to_file(DB_FILE) {
                            eprintln!("Failed to save db: {e}");
                            writeln!(stream, "ERR failed to save").ok();
                            continue;
                        }
                        writeln!(stream, "{added}").ok();
                    }
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            "SREM" => {
                if parts.len() < 3 {
                    writeln!(stream, "ERR usage: SREM key member [member ...]").ok();
                    continue;
                }
                let key = parts[1];
                let members: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();
                let mut store_guard = store.lock().unwrap();
                match store_guard.srem(key, members) {
                    Ok(removed) => {
                        if let Err(e) = store_guard.save_to_file(DB_FILE) {
                            eprintln!("Failed to save db: {e}");
                            writeln!(stream, "ERR failed to save").ok();
                            continue;
                        }
                        writeln!(stream, "{removed}").ok();
                    }
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            "SMEMBERS" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR usage: SMEMBERS key").ok();
                    continue;
                }
                let key = parts[1];
                let store_guard = store.lock().unwrap();
                match store_guard.smembers(key) {
                    Ok(members) => {
                        if members.is_empty() {
                            writeln!(stream, "(empty)").ok();
                        } else {
                            for m in members {
                                writeln!(stream, "{m}").ok();
                            }
                        }
                    }
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            "SISMEMBER" => {
                if parts.len() != 3 {
                    writeln!(stream, "ERR usage: SISMEMBER key member").ok();
                    continue;
                }
                let key = parts[1];
                let member = parts[2];
                let store_guard = store.lock().unwrap();
                match store_guard.sismember(key, member) {
                    Ok(true) => writeln!(stream, "1").ok(),
                    Ok(false) => writeln!(stream, "0").ok(),
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            // ---------- other commands (lists, strings, counters, etc.) ----------
            "LPUSH" => {
                if parts.len() < 3 {
                    writeln!(stream, "ERR usage: LPUSH key value [value ...]").ok();
                    continue;
                }
                let key = parts[1];
                let values: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();
                let mut store_guard = store.lock().unwrap();
                match store_guard.lpush(key, values) {
                    Ok(len) => {
                        if let Err(e) = store_guard.save_to_file(DB_FILE) {
                            eprintln!("Failed to save db: {e}");
                            writeln!(stream, "ERR failed to save").ok();
                            continue;
                        }
                        writeln!(stream, "{len}").ok();
                    }
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            "RPUSH" => {
                if parts.len() < 3 {
                    writeln!(stream, "ERR usage: RPUSH key value [value ...]").ok();
                    continue;
                }
                let key = parts[1];
                let values: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();
                let mut store_guard = store.lock().unwrap();
                match store_guard.rpush(key, values) {
                    Ok(len) => {
                        if let Err(e) = store_guard.save_to_file(DB_FILE) {
                            eprintln!("Failed to save db: {e}");
                            writeln!(stream, "ERR failed to save").ok();
                            continue;
                        }
                        writeln!(stream, "{len}").ok();
                    }
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            "LPOP" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR usage: LPOP key").ok();
                    continue;
                }
                let key = parts[1];
                let mut store_guard = store.lock().unwrap();
                match store_guard.lpop(key) {
                    Some(v) => {
                        if let Err(e) = store_guard.save_to_file(DB_FILE) {
                            eprintln!("Failed to save db: {e}");
                            writeln!(stream, "ERR failed to save").ok();
                            continue;
                        }
                        writeln!(stream, "{v}").ok();
                    }
                    None => writeln!(stream, "NIL").ok(),
                }
            }

            "RPOP" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR usage: RPOP key").ok();
                    continue;
                }
                let key = parts[1];
                let mut store_guard = store.lock().unwrap();
                match store_guard.rpop(key) {
                    Some(v) => {
                        if let Err(e) = store_guard.save_to_file(DB_FILE) {
                            eprintln!("Failed to save db: {e}");
                            writeln!(stream, "ERR failed to save").ok();
                            continue;
                        }
                        writeln!(stream, "{v}").ok();
                    }
                    None => writeln!(stream, "NIL").ok(),
                }
            }

            "LRANGE" => {
                if parts.len() != 4 {
                    writeln!(stream, "ERR usage: LRANGE key start end").ok();
                    continue;
                }
                let key = parts[1];
                let start: isize = match parts[2].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        writeln!(stream, "ERR start must be integer").ok();
                        continue;
                    }
                };
                let end: isize = match parts[3].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        writeln!(stream, "ERR end must be integer").ok();
                        continue;
                    }
                };
                let store_guard = store.lock().unwrap();
                match store_guard.lrange(key, start, end) {
                    Ok(vals) => {
                        if vals.is_empty() {
                            writeln!(stream, "(empty)").ok();
                        } else {
                            for v in vals {
                                writeln!(stream, "{v}").ok();
                            }
                        }
                    }
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            // ---------- Strings & basic commands ----------
            "GET" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR wrong number of arguments").ok();
                    continue;
                }
                let key = parts[1];
                let store_guard = store.lock().unwrap();
                if let Some(v) = store_guard.get(key) {
                    writeln!(stream, "VALUE {v}").ok();
                } else {
                    writeln!(stream, "NIL").ok();
                }
            }

            "SET" => {
                if parts.len() < 3 {
                    writeln!(stream, "ERR wrong number of arguments").ok();
                    continue;
                }
                let key = parts[1].to_string();
                let value = parts[2..].join(" ");
                let mut store_guard = store.lock().unwrap();
                store_guard.set_string(key, value);
                if let Err(e) = store_guard.save_to_file(DB_FILE) {
                    eprintln!("Failed to save db: {e}");
                    writeln!(stream, "ERR failed to save").ok();
                    continue;
                }
                writeln!(stream, "OK").ok();
            }

            "DEL" | "DELETE" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR wrong number of arguments").ok();
                    continue;
                }
                let key = parts[1];
                let mut store_guard = store.lock().unwrap();
                let existed = store_guard.delete(key);
                if let Err(e) = store_guard.save_to_file(DB_FILE) {
                    eprintln!("Failed to save db: {e}");
                    writeln!(stream, "ERR failed to save").ok();
                    continue;
                }
                writeln!(stream, "{}", if existed { 1 } else { 0 }).ok();
            }

            "EXISTS" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR wrong number of arguments").ok();
                    continue;
                }
                let key = parts[1];
                let store_guard = store.lock().unwrap();
                let exists = store_guard.exists(key);
                writeln!(stream, "{}", if exists { 1 } else { 0 }).ok();
            }

            "EXPIRE" => {
                if parts.len() != 3 {
                    writeln!(stream, "ERR usage: EXPIRE key seconds").ok();
                    continue;
                }
                let key = parts[1];
                let ttl: i64 = match parts[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        writeln!(stream, "ERR ttl must be integer").ok();
                        continue;
                    }
                };

                let mut store_guard = store.lock().unwrap();
                let ok = store_guard.expire(key, ttl);
                if ok {
                    if let Err(e) = store_guard.save_to_file(DB_FILE) {
                        eprintln!("Failed to save db: {e}");
                        writeln!(stream, "ERR failed to save").ok();
                        continue;
                    }
                }
                writeln!(stream, "{}", if ok { 1 } else { 0 }).ok();
            }

            "TTL" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR usage: TTL key").ok();
                    continue;
                }
                let key = parts[1];
                let store_guard = store.lock().unwrap();
                match store_guard.ttl(key) {
                    None => writeln!(stream, "-2").ok(),   // key does not exist
                    Some(-1) => writeln!(stream, "-1").ok(), // no expiry
                    Some(sec) => writeln!(stream, "{sec}").ok(),
                };
            }

            "INCR" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR usage: INCR key").ok();
                    continue;
                }
                let key = parts[1];
                let mut store_guard = store.lock().unwrap();
                match store_guard.incr(key, 1) {
                    Ok(new) => {
                        if let Err(e) = store_guard.save_to_file(DB_FILE) {
                            eprintln!("Failed to save db: {e}");
                            writeln!(stream, "ERR failed to save").ok();
                            continue;
                        }
                        writeln!(stream, "{new}").ok();
                    }
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            "DECR" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR usage: DECR key").ok();
                    continue;
                }
                let key = parts[1];
                let mut store_guard = store.lock().unwrap();
                match store_guard.incr(key, -1) {
                    Ok(new) => {
                        if let Err(e) = store_guard.save_to_file(DB_FILE) {
                            eprintln!("Failed to save db: {e}");
                            writeln!(stream, "ERR failed to save").ok();
                            continue;
                        }
                        writeln!(stream, "{new}").ok();
                    }
                    Err(e) => writeln!(stream, "ERR {e}").ok(),
                }
            }

            "FLUSHALL" => {
                let mut store_guard = store.lock().unwrap();
                store_guard.flush_all();
                if let Err(e) = store_guard.save_to_file(DB_FILE) {
                    eprintln!("Failed to save db: {e}");
                    writeln!(stream, "ERR failed to save").ok();
                    continue;
                }
                writeln!(stream, "OK").ok();
            }

            "QUIT" => {
                writeln!(stream, "BYE").ok();
                println!("Client quit: {:?}", peer);
                return;
            }

            _ => {
                writeln!(stream, "ERR unknown command").ok();
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let store = KvStore::from_file(DB_FILE)?;
    let store = Arc::new(Mutex::new(store));

    let listener = TcpListener::bind("127.0.0.1:6379")?;
    println!("mini_kv server listening on 127.0.0.1:6379");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    handle_client(stream, store_clone);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {e}");
            }
        }
    }

    Ok(())
}
