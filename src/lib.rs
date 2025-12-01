use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entry {
    value: String,
    // Unix timestamp (seconds) when key expires. None = no expiry.
    expires_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KvStore {
    map: HashMap<String, Entry>,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

impl KvStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::new());
        }

        let mut file = File::open(path)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        if buf.trim().is_empty() {
            return Ok(Self::new());
        }

        let map: HashMap<String, Entry> = serde_json::from_str(&buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(Self { map })
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let path = path.as_ref();
        let tmp_path = path.with_extension("tmp");

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp_path)?;

        let json = serde_json::to_string_pretty(&self.map)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        file.write_all(json.as_bytes())?;
        std::fs::rename(tmp_path, path)?;
        Ok(())
    }

    fn is_entry_alive(entry: &Entry) -> bool {
        if let Some(exp) = entry.expires_at {
            now_secs() < exp
        } else {
            true
        }
    }

    /// Basic SET: overwrites value and clears TTL.
    pub fn set(&mut self, key: String, value: String) {
        let entry = Entry {
            value,
            expires_at: None,
        };
        self.map.insert(key, entry);
    }

    /// GET returns cloned value (None if missing or expired).
    pub fn get(&self, key: &str) -> Option<String> {
        self.map.get(key).and_then(|entry| {
            if Self::is_entry_alive(entry) {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    /// DELETE returns true if key existed and was removed.
    pub fn delete(&mut self, key: &str) -> bool {
        self.map.remove(key).is_some()
    }

    /// EXISTS returns true if key exists and is not expired.
    pub fn exists(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// SET TTL in seconds. Returns true if key exists.
    pub fn expire(&mut self, key: &str, ttl_secs: i64) -> bool {
        if let Some(entry) = self.map.get_mut(key) {
            let ttl = if ttl_secs < 0 { 0 } else { ttl_secs };
            let exp = now_secs() + ttl;
            entry.expires_at = Some(exp);
            true
        } else {
            false
        }
    }

    /// TTL semantics:
    /// - None  => key not found
    /// - Some(-1) => key exists, no expiry
    /// - Some(n >= 0) => seconds remaining
    pub fn ttl(&self, key: &str) -> Option<i64> {
        let now = now_secs();
        self.map.get(key).and_then(|entry| {
            if let Some(exp) = entry.expires_at {
                if now >= exp {
                    None
                } else {
                    Some(exp - now)
                }
            } else {
                // exists, but no expiry
                Some(-1)
            }
        })
    }

    /// INCR/DECR functionality: delta can be +1 / -1 or any integer.
    /// - If key missing → treated as 0.
    /// - If value not an int → error.
    pub fn incr(&mut self, key: &str, delta: i64) -> Result<i64, String> {
        let entry = self
            .map
            .entry(key.to_string())
            .or_insert(Entry {
                value: "0".to_string(),
                expires_at: None,
            });

        // If expired, reset it to 0
        if let Some(exp) = entry.expires_at {
            if now_secs() >= exp {
                entry.value = "0".to_string();
                entry.expires_at = None;
            }
        }

        let current: i64 = entry
            .value
            .parse()
            .map_err(|_| "value is not an integer".to_string())?;

        let new_val = current + delta;
        entry.value = new_val.to_string();
        Ok(new_val)
    }

    /// KEYS with simple patterns:
    /// - "*" → all keys
    /// - "prefix*" → keys starting with prefix
    /// - anything else → exact match if exists
    pub fn keys(&self, pattern: &str) -> Vec<String> {
        let now = now_secs();

        if pattern == "*" {
            return self
                .map
                .iter()
                .filter_map(|(k, e)| {
                    if e.expires_at.map_or(true, |exp| now < exp) {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect();
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            return self
                .map
                .iter()
                .filter_map(|(k, e)| {
                    if k.starts_with(prefix)
                        && e.expires_at.map_or(true, |exp| now < exp)
                    {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect();
        }

        if self.exists(pattern) {
            vec![pattern.to_string()]
        } else {
            Vec::new()
        }
    }

    /// FLUSHALL – delete everything
    pub fn flush_all(&mut self) {
        self.map.clear();
    }
}
