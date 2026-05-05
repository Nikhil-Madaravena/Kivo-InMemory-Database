use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::resp::*;
use crate::cmd::{execute_command, build_info};
use crate::state::ServerState;

pub async fn handle_client(stream: TcpStream, state: Arc<ServerState>) {
    let peer = stream.peer_addr().ok();
    state.total_connections.fetch_add(1, Ordering::Relaxed);
    
    println!(r#"
  _  _______     ______  
 | |/ /_   _\ \ / / __ \ 
 | ' /  | |  \ V / |  | |
 |  <   | |   > <| |  | |
 | . \ _| |_ / . \ |__| |
 |_|\_\_____/_/ \_\____/ 
    IN-MEMORY DB
"#);
    println!("[kivo] Client connected: {peer:?}");

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // auth disabled when password is empty
    let auth_required = !state.config.password.is_empty();
    let mut authenticated = !auth_required;
    let mut in_tx = false;
    let mut tx_queue: Vec<Vec<String>> = Vec::new();

    loop {
        let raw_parts = match parse_command(&mut reader).await {
            Ok(p) => p,
            Err(e) => {
                if e != "EOF" { eprintln!("[kivo] Parse error from {peer:?}: {e}"); }
                println!("[kivo] Client disconnected: {peer:?}");
                return;
            }
        };
        if raw_parts.is_empty() { continue; }
        
        let mut parts = Vec::with_capacity(raw_parts.len());
        let mut valid_utf8 = true;
        for b in raw_parts {
            match String::from_utf8(b.to_vec()) {
                Ok(s) => parts.push(s),
                Err(_) => {
                    valid_utf8 = false;
                    break;
                }
            }
        }
        if !valid_utf8 {
            writer.write_all(resp_err("invalid utf-8").as_bytes()).await.ok();
            continue;
        }

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
                println!("[kivo] Client quit: {peer:?}");
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
