//! mini_kv storage engine
//!
//! Internally uses 16 independent shards, each protected by its own
//! `std::sync::RwLock`, so concurrent clients can operate on different
//! key-spaces in parallel without fighting over a single global lock.

use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::RwLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ---------------------------------------------------------------------------
// Time helpers
// ---------------------------------------------------------------------------

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum KvError {
    WrongType,
    NotInteger,
    Overflow,
    NotFound,
    SyntaxError,
    InvalidExpiry,
}

impl KvError {
    pub fn to_resp(&self) -> String {
        match self {
            KvError::WrongType =>
                "-WRONGTYPE Operation against a key holding the wrong kind of value\r\n".to_string(),
            KvError::NotInteger =>
                "-ERR value is not an integer or out of range\r\n".to_string(),
            KvError::Overflow =>
                "-ERR increment or decrement would overflow\r\n".to_string(),
            KvError::NotFound =>
                "-ERR no such key\r\n".to_string(),
            KvError::SyntaxError =>
                "-ERR syntax error\r\n".to_string(),
            KvError::InvalidExpiry =>
                "-ERR invalid expire time in command\r\n".to_string(),
        }
    }
}

pub type KvResult<T> = Result<T, KvError>;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DataType {
    String(String),
    List(VecDeque<String>),
    Hash(HashMap<String, String>),
}

impl DataType {
    pub fn type_name(&self) -> &'static str {
        match self {
            DataType::String(_) => "string",
            DataType::List(_)   => "list",
            DataType::Hash(_)   => "hash",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub value: DataType,
    /// Expiry as Unix timestamp in **milliseconds**. `None` = no expiry.
    pub expires_at_ms: Option<i64>,
    pub last_accessed_ms: i64,
}

impl Entry {
    fn new(value: DataType) -> Self {
        Self { value, expires_at_ms: None, last_accessed_ms: now_ms() }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at_ms.map_or(false, |exp| now_ms() >= exp)
    }

    /// Returns remaining TTL in milliseconds, -1 if no expiry, -2 if expired/missing.
    pub fn pttl(&self) -> i64 {
        match self.expires_at_ms {
            None => -1,
            Some(exp) => {
                let rem = exp - now_ms();
                if rem <= 0 { -2 } else { rem }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sharding
// ---------------------------------------------------------------------------

const NUM_SHARDS: usize = 16;

fn shard_idx(key: &str) -> usize {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    (h.finish() as usize) % NUM_SHARDS
}

#[derive(Default)]
struct Shard {
    entries: HashMap<String, Entry>,
}

impl Shard {
    /// Returns a live (non-expired) mutable reference, removing the key if expired.
    fn get_live_mut(&mut self, key: &str) -> Option<&mut Entry> {
        if let Some(e) = self.entries.get(key) {
            if e.is_expired() {
                self.entries.remove(key);
                return None;
            }
        }
        self.entries.get_mut(key)
    }

    /// Returns a live immutable reference without touching last_accessed.
    fn get_live(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key).filter(|e| !e.is_expired())
    }

    fn evict_lru_if_needed(&mut self, max_keys: usize) {
        if self.entries.len() >= max_keys {
            if let Some(k) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_accessed_ms)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&k);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Snapshot (for persistence)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub entries: HashMap<String, Entry>,
}

// ---------------------------------------------------------------------------
// KvStore
// ---------------------------------------------------------------------------

pub struct KvStore {
    shards: Vec<RwLock<Shard>>,
    max_keys_per_shard: usize,
    pub start_time: Instant,
}

impl KvStore {
    pub fn new(max_keys: usize) -> Self {
        let per_shard = (max_keys / NUM_SHARDS).max(1);
        let shards = (0..NUM_SHARDS).map(|_| RwLock::new(Shard::default())).collect();
        Self { shards, max_keys_per_shard: per_shard, start_time: Instant::now() }
    }

    // --- Persistence -------------------------------------------------------

    pub async fn load(path: &Path, max_keys: usize) -> std::io::Result<Self> {
        let store = Self::new(max_keys);
        if !path.exists() {
            return Ok(store);
        }
        let mut f = tokio::fs::File::open(path).await?;
        let mut buf = String::new();
        f.read_to_string(&mut buf).await?;
        if buf.trim().is_empty() {
            return Ok(store);
        }
        let snap: Snapshot = serde_json::from_str(&buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let now = now_ms();
        for (key, entry) in snap.entries {
            if entry.expires_at_ms.map_or(true, |exp| exp > now) {
                let idx = shard_idx(&key);
                store.shards[idx].write().unwrap().entries.insert(key, entry);
            }
        }
        println!("[mini_kv] Loaded snapshot from {:?}", path);
        Ok(store)
    }

    pub async fn save(&self, path: &Path) -> std::io::Result<()> {
        // Collect all live entries without holding locks during I/O.
        let mut all = HashMap::new();
        for shard in &self.shards {
            let g = shard.read().unwrap();
            for (k, v) in &g.entries {
                if !v.is_expired() {
                    all.insert(k.clone(), v.clone());
                }
            }
        }
        let snap = Snapshot { entries: all };
        let json = serde_json::to_string_pretty(&snap)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let tmp = path.with_extension("tmp");
        let mut f = OpenOptions::new().create(true).truncate(true).write(true)
            .open(&tmp).await?;
        f.write_all(json.as_bytes()).await?;
        f.flush().await?;
        tokio::fs::rename(&tmp, path).await?;
        Ok(())
    }

    // --- Helpers -----------------------------------------------------------

    fn write_shard(&self, key: &str) -> std::sync::RwLockWriteGuard<'_, Shard> {
        self.shards[shard_idx(key)].write().unwrap()
    }

    fn read_shard(&self, key: &str) -> std::sync::RwLockReadGuard<'_, Shard> {
        self.shards[shard_idx(key)].read().unwrap()
    }

    // --- Generic -----------------------------------------------------------

    pub fn del(&self, keys: &[String]) -> i64 {
        let mut count = 0i64;
        for key in keys {
            let mut s = self.write_shard(key);
            if s.entries.remove(key.as_str()).is_some() {
                count += 1;
            }
        }
        count
    }

    pub fn exists(&self, key: &str) -> bool {
        let s = self.read_shard(key);
        s.get_live(key).is_some()
    }

    pub fn type_of(&self, key: &str) -> &'static str {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None       => "none",
            Some(e)    => e.value.type_name(),
        }
    }

    pub fn expire(&self, key: &str, ttl_ms: i64) -> KvResult<bool> {
        if ttl_ms < 0 {
            return Err(KvError::InvalidExpiry);
        }
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(false),
            Some(e) => {
                e.expires_at_ms = Some(now_ms() + ttl_ms);
                Ok(true)
            }
        }
    }

    pub fn persist(&self, key: &str) -> bool {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => false,
            Some(e) => { e.expires_at_ms = None; true }
        }
    }

    pub fn ttl(&self, key: &str) -> i64 {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None    => -2,
            Some(e) => {
                let p = e.pttl();
                if p == -1 { -1 } else { p / 1000 }
            }
        }
    }

    pub fn pttl(&self, key: &str) -> i64 {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None    => -2,
            Some(e) => e.pttl(),
        }
    }

    pub fn rename(&self, from: &str, to: String) -> KvResult<()> {
        // Avoid deadlock: if from and to map to same shard, handle inline.
        if shard_idx(from) == shard_idx(&to) {
            let mut s = self.shards[shard_idx(from)].write().unwrap();
            let entry = s.get_live_mut(from)
                .ok_or(KvError::NotFound)?.clone();
            s.entries.remove(from);
            s.entries.insert(to, entry);
        } else {
            let entry = {
                let mut s = self.write_shard(from);
                s.get_live_mut(from).ok_or(KvError::NotFound)?.clone()
                // entry cloned; lock released here
            };
            {
                let mut s = self.write_shard(&to);
                s.entries.remove(from); // remove old if same bucket (can't happen here)
                s.entries.insert(to, entry);
            }
            self.write_shard(from).entries.remove(from);
        }
        Ok(())
    }

    pub fn db_size(&self) -> usize {
        let now = now_ms();
        self.shards.iter()
            .map(|s| s.read().unwrap().entries.values()
                .filter(|e| e.expires_at_ms.map_or(true, |exp| exp > now))
                .count())
            .sum()
    }

    pub fn keys(&self, pattern: &str) -> Vec<String> {
        let now = now_ms();
        let mut out = Vec::new();
        for shard in &self.shards {
            let s = shard.read().unwrap();
            for (k, e) in &s.entries {
                if e.expires_at_ms.map_or(true, |exp| exp > now) && glob_match(pattern, k) {
                    out.push(k.clone());
                }
            }
        }
        out
    }

    pub fn flush_all(&self) {
        for shard in &self.shards {
            shard.write().unwrap().entries.clear();
        }
    }

    pub fn purge_expired(&self) -> usize {
        let now = now_ms();
        let mut total = 0;
        for shard in &self.shards {
            let mut s = shard.write().unwrap();
            let before = s.entries.len();
            s.entries.retain(|_, e| e.expires_at_ms.map_or(true, |exp| exp > now));
            total += before - s.entries.len();
        }
        total
    }

    // --- Strings -----------------------------------------------------------

    pub fn set(&self, key: String, val: String, expire_ms: Option<i64>) -> KvResult<()> {
        if let Some(ms) = expire_ms {
            if ms <= 0 { return Err(KvError::InvalidExpiry); }
        }
        let mut s = self.write_shard(&key);
        s.evict_lru_if_needed(self.max_keys_per_shard);
        let mut e = Entry::new(DataType::String(val));
        e.expires_at_ms = expire_ms.map(|ms| now_ms() + ms);
        s.entries.insert(key, e);
        Ok(())
    }

    pub fn setnx(&self, key: String, val: String) -> bool {
        let mut s = self.write_shard(&key);
        if s.get_live(&key).is_some() { return false; }
        s.evict_lru_if_needed(self.max_keys_per_shard);
        s.entries.insert(key, Entry::new(DataType::String(val)));
        true
    }

    pub fn get(&self, key: &str) -> KvResult<Option<String>> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(None),
            Some(e) => {
                e.last_accessed_ms = now_ms();
                match &e.value {
                    DataType::String(v) => Ok(Some(v.clone())),
                    _ => Err(KvError::WrongType),
                }
            }
        }
    }

    pub fn getset(&self, key: String, new_val: String) -> KvResult<Option<String>> {
        let mut s = self.write_shard(&key);
        let old = match s.get_live_mut(&key) {
            None => None,
            Some(e) => match &e.value {
                DataType::String(v) => Some(v.clone()),
                _ => return Err(KvError::WrongType),
            },
        };
        s.evict_lru_if_needed(self.max_keys_per_shard);
        s.entries.insert(key, Entry::new(DataType::String(new_val)));
        Ok(old)
    }

    pub fn incr_by(&self, key: &str, delta: i64) -> KvResult<i64> {
        let mut s = self.write_shard(key);
        let (cur, exp) = match s.get_live_mut(key) {
            None => (0i64, None),
            Some(e) => {
                let exp = e.expires_at_ms;
                let cur = match &e.value {
                    DataType::String(v) => v.parse::<i64>().map_err(|_| KvError::NotInteger)?,
                    _ => return Err(KvError::WrongType),
                };
                (cur, exp)
            }
        };
        let new_val = cur.checked_add(delta).ok_or(KvError::Overflow)?;
        let mut e = Entry::new(DataType::String(new_val.to_string()));
        e.expires_at_ms = exp;
        s.entries.insert(key.to_string(), e);
        Ok(new_val)
    }

    pub fn append(&self, key: String, suffix: String) -> KvResult<usize> {
        let mut s = self.write_shard(&key);
        match s.get_live_mut(&key) {
            None => {
                let len = suffix.len();
                s.evict_lru_if_needed(self.max_keys_per_shard);
                s.entries.insert(key, Entry::new(DataType::String(suffix)));
                Ok(len)
            }
            Some(e) => {
                e.last_accessed_ms = now_ms();
                match &mut e.value {
                    DataType::String(v) => { v.push_str(&suffix); Ok(v.len()) }
                    _ => Err(KvError::WrongType),
                }
            }
        }
    }

    pub fn mset(&self, pairs: Vec<(String, String)>) {
        for (k, v) in pairs { let _ = self.set(k, v, None); }
    }

    pub fn mget(&self, keys: &[String]) -> Vec<Option<String>> {
        keys.iter().map(|k| self.get(k).unwrap_or(None)).collect()
    }

    // --- Hashes ------------------------------------------------------------

    pub fn hset(&self, key: String, field: String, val: String) -> KvResult<i64> {
        let mut s = self.write_shard(&key);
        let entry = s.entries.entry(key).or_insert_with(|| {
            Entry::new(DataType::Hash(HashMap::new()))
        });
        if entry.is_expired() {
            *entry = Entry::new(DataType::Hash(HashMap::new()));
        }
        entry.last_accessed_ms = now_ms();
        match &mut entry.value {
            DataType::Hash(h) => {
                let is_new = !h.contains_key(&field);
                h.insert(field, val);
                Ok(if is_new { 1 } else { 0 })
            }
            _ => Err(KvError::WrongType),
        }
    }

    pub fn hget(&self, key: &str, field: &str) -> KvResult<Option<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(None),
            Some(e) => match &e.value {
                DataType::Hash(h) => Ok(h.get(field).cloned()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn hdel(&self, key: &str, field: &str) -> KvResult<bool> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(false),
            Some(e) => match &mut e.value {
                DataType::Hash(h) => Ok(h.remove(field).is_some()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn hgetall(&self, key: &str) -> KvResult<Vec<(String, String)>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(vec![]),
            Some(e) => match &e.value {
                DataType::Hash(h) => Ok(h.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn hkeys(&self, key: &str) -> KvResult<Vec<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(vec![]),
            Some(e) => match &e.value {
                DataType::Hash(h) => Ok(h.keys().cloned().collect()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn hvals(&self, key: &str) -> KvResult<Vec<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(vec![]),
            Some(e) => match &e.value {
                DataType::Hash(h) => Ok(h.values().cloned().collect()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn hlen(&self, key: &str) -> KvResult<usize> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(0),
            Some(e) => match &e.value {
                DataType::Hash(h) => Ok(h.len()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn hexists(&self, key: &str, field: &str) -> KvResult<bool> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(false),
            Some(e) => match &e.value {
                DataType::Hash(h) => Ok(h.contains_key(field)),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn hmget(&self, key: &str, fields: &[String]) -> KvResult<Vec<Option<String>>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(fields.iter().map(|_| None).collect()),
            Some(e) => match &e.value {
                DataType::Hash(h) => Ok(fields.iter().map(|f| h.get(f).cloned()).collect()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    // --- Lists -------------------------------------------------------------

    pub fn lpush(&self, key: String, val: String) -> KvResult<usize> {
        self.list_push(key, val, true)
    }

    pub fn rpush(&self, key: String, val: String) -> KvResult<usize> {
        self.list_push(key, val, false)
    }

    fn list_push(&self, key: String, val: String, front: bool) -> KvResult<usize> {
        let mut s = self.write_shard(&key);
        let entry = s.entries.entry(key).or_insert_with(|| {
            Entry::new(DataType::List(VecDeque::new()))
        });
        if entry.is_expired() {
            *entry = Entry::new(DataType::List(VecDeque::new()));
        }
        entry.last_accessed_ms = now_ms();
        match &mut entry.value {
            DataType::List(l) => {
                if front { l.push_front(val); } else { l.push_back(val); }
                Ok(l.len())
            }
            _ => Err(KvError::WrongType),
        }
    }

    pub fn lpop(&self, key: &str) -> KvResult<Option<String>> {
        self.list_pop(key, true)
    }

    pub fn rpop(&self, key: &str) -> KvResult<Option<String>> {
        self.list_pop(key, false)
    }

    fn list_pop(&self, key: &str, front: bool) -> KvResult<Option<String>> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(None),
            Some(e) => {
                e.last_accessed_ms = now_ms();
                match &mut e.value {
                    DataType::List(l) => {
                        Ok(if front { l.pop_front() } else { l.pop_back() })
                    }
                    _ => Err(KvError::WrongType),
                }
            }
        }
    }

    pub fn llen(&self, key: &str) -> KvResult<usize> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(0),
            Some(e) => match &e.value {
                DataType::List(l) => Ok(l.len()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn lrange(&self, key: &str, start: i64, stop: i64) -> KvResult<Vec<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(vec![]),
            Some(e) => match &e.value {
                DataType::List(l) => {
                    let len = l.len() as i64;
                    let s = normalize_index(start, len);
                    let e = normalize_index(stop, len);
                    if s > e || s >= len { return Ok(vec![]); }
                    let end = (e + 1).min(len) as usize;
                    Ok(l.iter().skip(s as usize).take(end - s as usize).cloned().collect())
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn lindex(&self, key: &str, index: i64) -> KvResult<Option<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(None),
            Some(e) => match &e.value {
                DataType::List(l) => {
                    let len = l.len() as i64;
                    let idx = normalize_index(index, len);
                    Ok(if idx < 0 || idx >= len { None } else { l.get(idx as usize).cloned() })
                }
                _ => Err(KvError::WrongType),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Convert a Redis-style negative index to an absolute index.
fn normalize_index(idx: i64, len: i64) -> i64 {
    if idx < 0 { (len + idx).max(0) } else { idx }
}

/// Glob pattern matching supporting `*` (any sequence) and `?` (any char).
pub fn glob_match(pattern: &str, s: &str) -> bool {
    glob_inner(pattern.as_bytes(), s.as_bytes())
}

fn glob_inner(p: &[u8], s: &[u8]) -> bool {
    match (p.first(), s.first()) {
        (None, None)         => true,
        (None, _)            => false,
        (Some(b'*'), _)      => glob_inner(&p[1..], s) || (!s.is_empty() && glob_inner(p, &s[1..])),
        (_, None)            => false,
        (Some(b'?'), _)      => glob_inner(&p[1..], &s[1..]),
        (Some(pc), Some(sc)) => pc == sc && glob_inner(&p[1..], &s[1..]),
    }
}
