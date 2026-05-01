//! mini_kv storage engine
//!
//! Internally uses 16 independent shards, each protected by its own
//! `std::sync::RwLock`, so concurrent clients can operate on different
//! key-spaces in parallel without fighting over a single global lock.

use std::collections::{HashMap, HashSet, VecDeque};
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
    Set(HashSet<String>),
    /// Sorted set: member -> score. BTreeMap<score_bits, member> for ordering.
    ZSet(HashMap<String, f64>),
}

impl DataType {
    pub fn type_name(&self) -> &'static str {
        match self {
            DataType::String(_) => "string",
            DataType::List(_)   => "list",
            DataType::Hash(_)   => "hash",
            DataType::Set(_)    => "set",
            DataType::ZSet(_)   => "zset",
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

    // --- List extras -------------------------------------------------------

    pub fn lset(&self, key: &str, index: i64, val: String) -> KvResult<()> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Err(KvError::NotFound),
            Some(e) => match &mut e.value {
                DataType::List(l) => {
                    let len = l.len() as i64;
                    let idx = normalize_index(index, len);
                    if idx < 0 || idx >= len { return Err(KvError::NotFound); }
                    l[idx as usize] = val;
                    Ok(())
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn lrem(&self, key: &str, count: i64, val: &str) -> KvResult<i64> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(0),
            Some(e) => match &mut e.value {
                DataType::List(l) => {
                    let mut removed = 0i64;
                    if count == 0 {
                        l.retain(|v| { if v == val { removed += 1; false } else { true } });
                    } else if count > 0 {
                        let mut new = VecDeque::with_capacity(l.len());
                        for v in l.drain(..) {
                            if v == val && removed < count { removed += 1; } else { new.push_back(v); }
                        }
                        *l = new;
                    } else {
                        let mut rev: Vec<_> = l.drain(..).collect();
                        rev.retain(|v| if v == val && removed < count.abs() { removed += 1; false } else { true });
                        *l = rev.into_iter().collect();
                    }
                    Ok(removed)
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn ltrim(&self, key: &str, start: i64, stop: i64) -> KvResult<()> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(()),
            Some(e) => match &mut e.value {
                DataType::List(l) => {
                    let len = l.len() as i64;
                    let s_idx = normalize_index(start, len).max(0) as usize;
                    let e_idx = (normalize_index(stop, len) + 1).min(len) as usize;
                    if s_idx >= e_idx { l.clear(); return Ok(()); }
                    let trimmed: VecDeque<_> = l.drain(s_idx..e_idx).collect();
                    *l = trimmed;
                    Ok(())
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    // --- Sets ---------------------------------------------------------------

    pub fn sadd(&self, key: String, members: &[String]) -> KvResult<i64> {
        let mut s = self.write_shard(&key);
        let entry = s.entries.entry(key).or_insert_with(|| Entry::new(DataType::Set(HashSet::new())));
        if entry.is_expired() { *entry = Entry::new(DataType::Set(HashSet::new())); }
        entry.last_accessed_ms = now_ms();
        match &mut entry.value {
            DataType::Set(set) => {
                let added = members.iter().filter(|m| set.insert((*m).clone())).count();
                Ok(added as i64)
            }
            _ => Err(KvError::WrongType),
        }
    }

    pub fn srem(&self, key: &str, members: &[String]) -> KvResult<i64> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(0),
            Some(e) => match &mut e.value {
                DataType::Set(set) => Ok(members.iter().filter(|m| set.remove(m.as_str())).count() as i64),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn smembers(&self, key: &str) -> KvResult<Vec<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(vec![]),
            Some(e) => match &e.value {
                DataType::Set(set) => Ok(set.iter().cloned().collect()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn scard(&self, key: &str) -> KvResult<usize> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(0),
            Some(e) => match &e.value {
                DataType::Set(set) => Ok(set.len()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn sismember(&self, key: &str, member: &str) -> KvResult<bool> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(false),
            Some(e) => match &e.value {
                DataType::Set(set) => Ok(set.contains(member)),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn spop(&self, key: &str) -> KvResult<Option<String>> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(None),
            Some(e) => match &mut e.value {
                DataType::Set(set) => {
                    // pick pseudo-random element using time as noise
                    let pick = (now_ms() as usize).wrapping_rem(set.len().max(1));
                    let key = set.iter().nth(pick).cloned();
                    if let Some(ref k) = key { set.remove(k); }
                    Ok(key)
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn srandmember(&self, key: &str) -> KvResult<Option<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(None),
            Some(e) => match &e.value {
                DataType::Set(set) => {
                    let pick = (now_ms() as usize).wrapping_rem(set.len().max(1));
                    Ok(set.iter().nth(pick).cloned())
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn smove(&self, src: &str, dst: String, member: String) -> KvResult<bool> {
        // Remove from src first
        let removed = {
            let mut s = self.write_shard(src);
            match s.get_live_mut(src) {
                None => return Ok(false),
                Some(e) => match &mut e.value {
                    DataType::Set(set) => set.remove(&member),
                    _ => return Err(KvError::WrongType),
                },
            }
        };
        if !removed { return Ok(false); }
        // Add to dst
        self.sadd(dst, &[member])?;
        Ok(true)
    }

    fn get_set_snapshot(&self, key: &str) -> KvResult<HashSet<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(HashSet::new()),
            Some(e) => match &e.value {
                DataType::Set(set) => Ok(set.clone()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn sunion(&self, keys: &[String]) -> KvResult<Vec<String>> {
        let mut result = HashSet::new();
        for k in keys { result.extend(self.get_set_snapshot(k)?); }
        Ok(result.into_iter().collect())
    }

    pub fn sinter(&self, keys: &[String]) -> KvResult<Vec<String>> {
        if keys.is_empty() { return Ok(vec![]); }
        let mut result = self.get_set_snapshot(&keys[0])?;
        for k in &keys[1..] { let s = self.get_set_snapshot(k)?; result.retain(|m| s.contains(m)); }
        Ok(result.into_iter().collect())
    }

    pub fn sdiff(&self, keys: &[String]) -> KvResult<Vec<String>> {
        if keys.is_empty() { return Ok(vec![]); }
        let mut result = self.get_set_snapshot(&keys[0])?;
        for k in &keys[1..] { let s = self.get_set_snapshot(k)?; result.retain(|m| !s.contains(m)); }
        Ok(result.into_iter().collect())
    }

    pub fn sunionstore(&self, dst: String, keys: &[String]) -> KvResult<usize> {
        let members: Vec<String> = self.sunion(keys)?;
        let len = members.len();
        let mut s = self.write_shard(&dst);
        s.entries.insert(dst, Entry::new(DataType::Set(members.into_iter().collect())));
        Ok(len)
    }

    pub fn sinterstore(&self, dst: String, keys: &[String]) -> KvResult<usize> {
        let members: Vec<String> = self.sinter(keys)?;
        let len = members.len();
        let mut s = self.write_shard(&dst);
        s.entries.insert(dst, Entry::new(DataType::Set(members.into_iter().collect())));
        Ok(len)
    }

    pub fn sdiffstore(&self, dst: String, keys: &[String]) -> KvResult<usize> {
        let members: Vec<String> = self.sdiff(keys)?;
        let len = members.len();
        let mut s = self.write_shard(&dst);
        s.entries.insert(dst, Entry::new(DataType::Set(members.into_iter().collect())));
        Ok(len)
    }

    // --- Sorted Sets --------------------------------------------------------

    /// Returns sorted (score, member) pairs for a ZSet.
    fn zset_sorted(map: &HashMap<String, f64>) -> Vec<(f64, &str)> {
        let mut v: Vec<(f64, &str)> = map.iter().map(|(m, &sc)| (sc, m.as_str())).collect();
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(b.1)));
        v
    }

    pub fn zadd(&self, key: String, pairs: &[(f64, String)]) -> KvResult<i64> {
        let mut s = self.write_shard(&key);
        let entry = s.entries.entry(key).or_insert_with(|| Entry::new(DataType::ZSet(HashMap::new())));
        if entry.is_expired() { *entry = Entry::new(DataType::ZSet(HashMap::new())); }
        entry.last_accessed_ms = now_ms();
        match &mut entry.value {
            DataType::ZSet(map) => {
                let added = pairs.iter().filter(|(_, m)| !map.contains_key(m)).count();
                for (sc, m) in pairs { map.insert(m.clone(), *sc); }
                Ok(added as i64)
            }
            _ => Err(KvError::WrongType),
        }
    }

    pub fn zrem(&self, key: &str, members: &[String]) -> KvResult<i64> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(0),
            Some(e) => match &mut e.value {
                DataType::ZSet(map) => Ok(members.iter().filter(|m| map.remove(m.as_str()).is_some()).count() as i64),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn zscore(&self, key: &str, member: &str) -> KvResult<Option<f64>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(None),
            Some(e) => match &e.value {
                DataType::ZSet(map) => Ok(map.get(member).copied()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn zincrby(&self, key: String, delta: f64, member: String) -> KvResult<f64> {
        let mut s = self.write_shard(&key);
        let entry = s.entries.entry(key).or_insert_with(|| Entry::new(DataType::ZSet(HashMap::new())));
        if entry.is_expired() { *entry = Entry::new(DataType::ZSet(HashMap::new())); }
        match &mut entry.value {
            DataType::ZSet(map) => {
                let score = map.entry(member).or_insert(0.0);
                *score += delta;
                Ok(*score)
            }
            _ => Err(KvError::WrongType),
        }
    }

    pub fn zcard(&self, key: &str) -> KvResult<usize> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(0),
            Some(e) => match &e.value {
                DataType::ZSet(map) => Ok(map.len()),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn zrank(&self, key: &str, member: &str, reverse: bool) -> KvResult<Option<i64>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(None),
            Some(e) => match &e.value {
                DataType::ZSet(map) => {
                    let sorted = Self::zset_sorted(map);
                    let pos = sorted.iter().position(|(_, m)| *m == member);
                    Ok(pos.map(|i| if reverse { (sorted.len() - 1 - i) as i64 } else { i as i64 }))
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn zrange(&self, key: &str, start: i64, stop: i64, rev: bool, with_scores: bool) -> KvResult<Vec<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(vec![]),
            Some(e) => match &e.value {
                DataType::ZSet(map) => {
                    let mut sorted = Self::zset_sorted(map);
                    if rev { sorted.reverse(); }
                    let len = sorted.len() as i64;
                    let s_idx = normalize_index(start, len).max(0) as usize;
                    let e_idx = (normalize_index(stop, len) + 1).min(len) as usize;
                    if s_idx >= e_idx { return Ok(vec![]); }
                    let mut out = vec![];
                    for (sc, m) in &sorted[s_idx..e_idx] {
                        out.push(m.to_string());
                        if with_scores { out.push(format_score(*sc)); }
                    }
                    Ok(out)
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn zrangebyscore(&self, key: &str, min: f64, max: f64, with_scores: bool) -> KvResult<Vec<String>> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(vec![]),
            Some(e) => match &e.value {
                DataType::ZSet(map) => {
                    let sorted = Self::zset_sorted(map);
                    let mut out = vec![];
                    for (sc, m) in sorted.into_iter().filter(|(sc, _)| *sc >= min && *sc <= max) {
                        out.push(m.to_string());
                        if with_scores { out.push(format_score(sc)); }
                    }
                    Ok(out)
                }
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn zcount(&self, key: &str, min: f64, max: f64) -> KvResult<i64> {
        let s = self.read_shard(key);
        match s.get_live(key) {
            None => Ok(0),
            Some(e) => match &e.value {
                DataType::ZSet(map) => Ok(map.values().filter(|&&sc| sc >= min && sc <= max).count() as i64),
                _ => Err(KvError::WrongType),
            },
        }
    }

    pub fn zpopmin(&self, key: &str) -> KvResult<Vec<String>> {
        self.zpop(key, false)
    }

    pub fn zpopmax(&self, key: &str) -> KvResult<Vec<String>> {
        self.zpop(key, true)
    }

    fn zpop(&self, key: &str, max: bool) -> KvResult<Vec<String>> {
        let mut s = self.write_shard(key);
        match s.get_live_mut(key) {
            None => Ok(vec![]),
            Some(e) => match &mut e.value {
                DataType::ZSet(map) => {
                    if map.is_empty() { return Ok(vec![]); }
                    let sorted = Self::zset_sorted(map);
                    let (sc, m) = if max { *sorted.last().unwrap() } else { *sorted.first().unwrap() };
                    let m = m.to_string();
                    map.remove(&m);
                    Ok(vec![m, format_score(sc)])
                }
                _ => Err(KvError::WrongType),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Format a float score the Redis way: integers drop the trailing ".0".
pub fn format_score(sc: f64) -> String {
    if sc == f64::INFINITY        { return "+inf".to_string(); }
    if sc == f64::NEG_INFINITY    { return "-inf".to_string(); }
    if sc.fract() == 0.0 && sc.abs() < 1e15 { format!("{}", sc as i64) }
    else                          { format!("{sc}") }
}

/// Parse a score token, accepting "+inf", "-inf", and numeric strings.
pub fn parse_score(s: &str) -> Option<f64> {
    match s {
        "+inf" | "inf"  => Some(f64::INFINITY),
        "-inf"          => Some(f64::NEG_INFINITY),
        _               => s.parse().ok(),
    }
}

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
