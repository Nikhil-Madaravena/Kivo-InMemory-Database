use kivo::{KvStore, KvError, parse_score};
use crate::resp::*;
use crate::state::ServerState;
use std::sync::atomic::Ordering;

pub fn execute_command(store: &KvStore, cmd: &str, parts: &[String]) -> String {
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

        "LSET"    => {
            args_eq!(4);
            let idx: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.lset(&parts[1], idx, parts[3].clone()), |_| resp_ok().to_string())
        }
        "LREM"    => {
            args_eq!(4);
            let count: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.lrem(&parts[1], count, &parts[3]), |n| resp_int(n))
        }
        "LTRIM"   => {
            args_eq!(4);
            let start: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            let stop:  i64 = match parts[3].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            resp_kv_result(store.ltrim(&parts[1], start, stop), |_| resp_ok().to_string())
        }

        // --- Sets ---
        "SADD"    => {
            args_ge!(3);
            resp_kv_result(store.sadd(parts[1].clone(), &parts[2..]), |n| resp_int(n))
        }
        "SREM"    => {
            args_ge!(3);
            resp_kv_result(store.srem(&parts[1], &parts[2..]), |n| resp_int(n))
        }
        "SMEMBERS"    => { args_eq!(2); resp_kv_result(store.smembers(&parts[1]), |v| resp_array(&v)) }
        "SCARD"       => { args_eq!(2); resp_kv_result(store.scard(&parts[1]),    |n| resp_int(n as i64)) }
        "SISMEMBER"   => {
            args_eq!(3);
            resp_kv_result(store.sismember(&parts[1], &parts[2]), |b| resp_int(b as i64))
        }
        "SPOP"        => { args_eq!(2); resp_kv_result(store.spop(&parts[1]),         resp_opt_bulk) }
        "SRANDMEMBER" => { args_eq!(2); resp_kv_result(store.srandmember(&parts[1]),  resp_opt_bulk) }
        "SMOVE"       => {
            args_eq!(4);
            resp_kv_result(store.smove(&parts[1], parts[2].clone(), parts[3].clone()), |b| resp_int(b as i64))
        }
        "SUNION"  => { args_ge!(2); resp_kv_result(store.sunion(&parts[1..]),  |v| resp_array(&v)) }
        "SINTER"  => { args_ge!(2); resp_kv_result(store.sinter(&parts[1..]),  |v| resp_array(&v)) }
        "SDIFF"   => { args_ge!(2); resp_kv_result(store.sdiff(&parts[1..]),   |v| resp_array(&v)) }
        "SUNIONSTORE" => {
            args_ge!(3);
            resp_kv_result(store.sunionstore(parts[1].clone(), &parts[2..]), |n| resp_int(n as i64))
        }
        "SINTERSTORE" => {
            args_ge!(3);
            resp_kv_result(store.sinterstore(parts[1].clone(), &parts[2..]), |n| resp_int(n as i64))
        }
        "SDIFFSTORE"  => {
            args_ge!(3);
            resp_kv_result(store.sdiffstore(parts[1].clone(), &parts[2..]), |n| resp_int(n as i64))
        }

        // --- Sorted Sets ---
        "ZADD" => {
            // ZADD key [NX|XX] [GT|LT] [CH] [INCR] score member [score member …]
            args_ge!(4);
            if (parts.len() - 2) % 2 != 0 {
                return resp_err("wrong number of arguments for 'zadd' command");
            }
            let pairs: Vec<(f64, String)> = {
                let mut v = vec![];
                let mut i = 2;
                while i + 1 < parts.len() {
                    let sc = match parse_score(&parts[i]) {
                        Some(s) => s,
                        None    => return resp_err("value is not a valid float"),
                    };
                    v.push((sc, parts[i + 1].clone()));
                    i += 2;
                }
                v
            };
            resp_kv_result(store.zadd(parts[1].clone(), &pairs), |n| resp_int(n))
        }
        "ZREM"    => { args_ge!(3); resp_kv_result(store.zrem(&parts[1], &parts[2..]), |n| resp_int(n)) }
        "ZSCORE"  => {
            args_eq!(3);
            resp_kv_result(store.zscore(&parts[1], &parts[2]), |sc| {
                match sc { None => resp_nil().to_string(), Some(s) => resp_bulk(&kivo::format_score(s)) }
            })
        }
        "ZINCRBY" => {
            args_eq!(4);
            let delta = match parse_score(&parts[2]) { Some(v) => v, None => return resp_err("value is not a valid float") };
            resp_kv_result(store.zincrby(parts[1].clone(), delta, parts[3].clone()), |s| resp_bulk(&kivo::format_score(s)))
        }
        "ZCARD"      => { args_eq!(2); resp_kv_result(store.zcard(&parts[1]), |n| resp_int(n as i64)) }
        "ZRANK"      => {
            args_eq!(3);
            resp_kv_result(store.zrank(&parts[1], &parts[2], false), |r| match r { None => resp_nil().to_string(), Some(n) => resp_int(n) })
        }
        "ZREVRANK"   => {
            args_eq!(3);
            resp_kv_result(store.zrank(&parts[1], &parts[2], true),  |r| match r { None => resp_nil().to_string(), Some(n) => resp_int(n) })
        }
        "ZRANGE" => {
            args_ge!(4);
            let start: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            let stop:  i64 = match parts[3].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            let with_scores = parts.get(4).map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false);
            resp_kv_result(store.zrange(&parts[1], start, stop, false, with_scores), |v| resp_array(&v))
        }
        "ZREVRANGE" => {
            args_ge!(4);
            let start: i64 = match parts[2].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            let stop:  i64 = match parts[3].parse() { Ok(v) => v, Err(_) => return KvError::NotInteger.to_resp() };
            let with_scores = parts.get(4).map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false);
            resp_kv_result(store.zrange(&parts[1], start, stop, true, with_scores), |v| resp_array(&v))
        }
        "ZRANGEBYSCORE" => {
            args_ge!(4);
            let min = match parse_score(&parts[2]) { Some(v) => v, None => return resp_err("min is not a float") };
            let max = match parse_score(&parts[3]) { Some(v) => v, None => return resp_err("max is not a float") };
            let with_scores = parts.get(4).map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false);
            resp_kv_result(store.zrangebyscore(&parts[1], min, max, with_scores), |v| resp_array(&v))
        }
        "ZREVRANGEBYSCORE" => {
            args_ge!(4);
            // Redis reverses min/max args in ZREVRANGEBYSCORE
            let max = match parse_score(&parts[2]) { Some(v) => v, None => return resp_err("max is not a float") };
            let min = match parse_score(&parts[3]) { Some(v) => v, None => return resp_err("min is not a float") };
            let with_scores = parts.get(4).map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false);
            resp_kv_result(store.zrangebyscore(&parts[1], min, max, with_scores), |mut v| { v.reverse(); resp_array(&v) })
        }
        "ZCOUNT"  => {
            args_eq!(4);
            let min = match parse_score(&parts[2]) { Some(v) => v, None => return resp_err("min is not a float") };
            let max = match parse_score(&parts[3]) { Some(v) => v, None => return resp_err("max is not a float") };
            resp_kv_result(store.zcount(&parts[1], min, max), |n| resp_int(n))
        }
        "ZPOPMIN" => { args_eq!(2); resp_kv_result(store.zpopmin(&parts[1]), |v| resp_array(&v)) }
        "ZPOPMAX" => { args_eq!(2); resp_kv_result(store.zpopmax(&parts[1]), |v| resp_array(&v)) }

        // --- Cursor-based iteration (simplified: always returns all in one shot) ---
        "SCAN" => {
            // SCAN cursor [MATCH pattern] [COUNT count] [TYPE type]
            let mut pattern = "*".to_string();
            let mut type_filter: Option<String> = None;
            let mut i = 2;
            while i + 1 < parts.len() {
                match parts[i].to_uppercase().as_str() {
                    "MATCH" => { pattern = parts[i + 1].clone(); i += 2; }
                    "COUNT" => { i += 2; } // hint; we always return all
                    "TYPE"  => { type_filter = Some(parts[i + 1].to_lowercase()); i += 2; }
                    _       => { i += 1; }
                }
            }
            let keys = store.keys(&pattern);
            let keys: Vec<String> = match type_filter {
                None    => keys,
                Some(t) => keys.into_iter().filter(|k| store.type_of(k) == t).collect(),
            };
            // Return cursor "0" (done) + array of keys
            format!("*2\r\n$1\r\n0\r\n{}", resp_array(&keys))
        }
        "HSCAN" => {
            args_ge!(2);
            let all = store.hgetall(&parts[1]).unwrap_or_default();
            let flat: Vec<String> = all.into_iter().flat_map(|(k, v)| [k, v]).collect();
            format!("*2\r\n$1\r\n0\r\n{}", resp_array(&flat))
        }
        "SSCAN" => {
            args_ge!(2);
            let members = store.smembers(&parts[1]).unwrap_or_default();
            format!("*2\r\n$1\r\n0\r\n{}", resp_array(&members))
        }
        "ZSCAN" => {
            args_ge!(2);
            let members = store.zrange(&parts[1], 0, -1, false, true).unwrap_or_default();
            format!("*2\r\n$1\r\n0\r\n{}", resp_array(&members))
        }

        _ => resp_err(&format!("unknown command `{}`, with args beginning with: {}", cmd.to_lowercase(),
            parts[1..].iter().take(3).map(|s| format!("`{s}`")).collect::<Vec<_>>().join(", "))),
    }
}

pub fn build_info(state: &ServerState) -> String {
    let uptime_secs = state.store.start_time.elapsed().as_secs();
    let total_keys  = state.store.db_size();
    let cmds        = state.total_commands.load(Ordering::Relaxed);
    let conns       = state.total_connections.load(Ordering::Relaxed);

    let body = format!(
"# Server\r\nredis_version:7.0.0-kivo\r\nredis_mode:standalone\r\nnos:rust\r\n\
arch_bits:64\r\nuptime_in_seconds:{uptime_secs}\r\n\
\r\n# Clients\r\ntotal_connections_received:{conns}\r\n\
\r\n# Stats\r\ntotal_commands_processed:{cmds}\r\n\
\r\n# Keyspace\r\ndb0:keys={total_keys},expires=0,avg_ttl=0\r\n"
    );
    resp_bulk(&body)
}
