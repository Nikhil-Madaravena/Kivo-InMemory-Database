use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Value {
    String(String),
    List(VecDeque<String>),
    // future: Hash, Set, etc.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub value: Value,
    pub expires_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KvStore {
    pub map: HashMap<String, Entry>,
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

    // -------------------------
    // String operations
    // -------------------------

    /// Basic SET: overwrites value and clears TTL.
    pub fn set_string(&mut self, key: String, value: String) {
        let entry = Entry {
            value: Value::String(value),
            expires_at: None,
        };
        self.map.insert(key, entry);
    }

    /// GET returns cloned string value if value type is String (else None)
    pub fn get(&self, key: &str) -> Option<String> {
        self.map.get(key).and_then(|entry| {
            if !Self::is_entry_alive(entry) {
                return None;
            }
            match &entry.value {
                Value::String(s) => Some(s.clone()),
                _ => None,
            }
        })
    }

    pub fn delete(&mut self, key: &str) -> bool {
        self.map.remove(key).is_some()
    }

    pub fn exists(&self, key: &str) -> bool {
        self.map.get(key).map_or(false, |entry| Self::is_entry_alive(entry))
    }

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
    /// - None => key not found
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
                Some(-1)
            }
        })
    }

    /// Basic INCR/DECR: treats missing as 0, errors if not integer
    pub fn incr(&mut self, key: &str, delta: i64) -> Result<i64, String> {
        // If key exists but expired, treat as missing
        if let Some(entry) = self.map.get_mut(key) {
            if !Self::is_entry_alive(entry) {
                self.map.remove(key);
            }
        }

        let entry = self
            .map
            .entry(key.to_string())
            .or_insert(Entry {
                value: Value::String("0".to_string()),
                expires_at: None,
            });

        match &mut entry.value {
            Value::String(s) => {
                let cur: i64 = s.parse().map_err(|_| "value is not an integer".to_string())?;
                let new = cur + delta;
                *s = new.to_string();
                Ok(new)
            }
            _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value".into()),
        }
    }

    // -------------------------
    // LIST operations
    // -------------------------

    /// Internal helper: get mutable entry only if alive (removes expired entries)
    fn get_entry_mut_if_alive(&mut self, key: &str) -> Option<&mut Entry> {
        if let Some(entry) = self.map.get_mut(key) {
            if !Self::is_entry_alive(entry) {
                // expired: remove and return None
                self.map.remove(key);
                return None;
            }
            Some(entry)
        } else {
            None
        }
    }

    /// LPUSH: push values to head (left). Returns new length or Err if wrong type.
    pub fn lpush(&mut self, key: &str, values: Vec<String>) -> Result<usize, String> {
        if let Some(entry) = self.get_entry_mut_if_alive(key) {
            match &mut entry.value {
                Value::List(list) => {
                    for val in values.into_iter() {
                        list.push_front(val);
                    }
                    return Ok(list.len());
                }
                _ => return Err("WRONGTYPE Operation against a key holding the wrong kind of value".into()),
            }
        }

        // create new list
        let mut dq = VecDeque::new();
        for val in values.into_iter() {
            dq.push_front(val);
        }
        let entry = Entry {
            value: Value::List(dq),
            expires_at: None,
        };
        self.map.insert(key.to_string(), entry);
        match self.map.get(key) {
            Some(e) => match &e.value {
                Value::List(list) => Ok(list.len()),
                _ => unreachable!(),
            },
            None => Ok(0),
        }
    }

    /// RPUSH: push values to tail (right). Return new length or Err.
    pub fn rpush(&mut self, key: &str, values: Vec<String>) -> Result<usize, String> {
        if let Some(entry) = self.get_entry_mut_if_alive(key) {
            match &mut entry.value {
                Value::List(list) => {
                    for val in values.into_iter() {
                        list.push_back(val);
                    }
                    return Ok(list.len());
                }
                _ => return Err("WRONGTYPE Operation against a key holding the wrong kind of value".into()),
            }
        }

        let mut dq = VecDeque::new();
        for val in values.into_iter() {
            dq.push_back(val);
        }
        let entry = Entry {
            value: Value::List(dq),
            expires_at: None,
        };
        self.map.insert(key.to_string(), entry);
        match self.map.get(key) {
            Some(e) => match &e.value {
                Value::List(list) => Ok(list.len()),
                _ => unreachable!(),
            },
            None => Ok(0),
        }
    }

    /// LPOP: pop from head. Returns Some(value) or None
    pub fn lpop(&mut self, key: &str) -> Option<String> {
        if let Some(entry) = self.get_entry_mut_if_alive(key) {
            match &mut entry.value {
                Value::List(list) => {
                    let v = list.pop_front();
                    if list.is_empty() {
                        self.map.remove(key);
                    }
                    v
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// RPOP: pop from tail.
    pub fn rpop(&mut self, key: &str) -> Option<String> {
        if let Some(entry) = self.get_entry_mut_if_alive(key) {
            match &mut entry.value {
                Value::List(list) => {
                    let v = list.pop_back();
                    if list.is_empty() {
                        self.map.remove(key);
                    }
                    v
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// LRANGE: start and end inclusive. Support negative indices.
    /// Returns Vec<String> (may be empty).
    pub fn lrange(&self, key: &str, start: isize, end: isize) -> Result<Vec<String>, String> {
        if let Some(entry) = self.map.get(key) {
            if !Self::is_entry_alive(entry) {
                return Ok(vec![]);
            }
            match &entry.value {
                Value::List(list) => {
                    let len = list.len() as isize;
                    if len == 0 {
                        return Ok(vec![]);
                    }

                    let mut s = if start < 0 { len + start } else { start };
                    let mut e = if end < 0 { len + end } else { end };

                    if s < 0 { s = 0; }
                    if e < 0 { return Ok(vec![]); } // end before start after normalization

                    if s as usize >= list.len() || s > e {
                        return Ok(vec![]);
                    }
                    if e >= len { e = len - 1; }

                    let s_us = s as usize;
                    let e_us = e as usize;

                    let mut out = Vec::new();
                    for i in s_us..=e_us {
                        if let Some(val) = list.get(i) {
                            out.push(val.clone());
                        }
                    }
                    Ok(out)
                }
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value".into()),
            }
        } else {
            Ok(vec![])
        }
    }

    /// FLUSHALL – delete everything
    pub fn flush_all(&mut self) {
        self.map.clear();
    }
}
