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

        // Strip \r\n
        let line = line.trim_end_matches(&['\r', '\n'][..]);

        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_uppercase();

        match cmd.as_str() {
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

                {
                    let mut store_guard = store.lock().unwrap();
                    store_guard.set(key, value);
                    if let Err(e) = store_guard.save_to_file(DB_FILE) {
                        eprintln!("Failed to save db: {e}");
                        writeln!(stream, "ERR failed to save").ok();
                        continue;
                    }
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
                    Err(e) => {
                        writeln!(stream, "ERR {e}").ok();
                    }
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
                    Err(e) => {
                        writeln!(stream, "ERR {e}").ok();
                    }
                }
            }

            "KEYS" => {
                if parts.len() != 2 {
                    writeln!(stream, "ERR usage: KEYS pattern").ok();
                    continue;
                }
                let pattern = parts[1];
                let store_guard = store.lock().unwrap();
                let keys = store_guard.keys(pattern);
                if keys.is_empty() {
                    writeln!(stream, "(empty)").ok();
                } else {
                    // simple space-separated output
                    let joined = keys.join(" ");
                    writeln!(stream, "{joined}").ok();
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
