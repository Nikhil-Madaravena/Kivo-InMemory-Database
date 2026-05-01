# mini_kv

A lightweight, Redis-compatible in-memory key-value store written in Rust.

`mini_kv` speaks the **Redis Serialization Protocol (RESP)**, so it works out-of-the-box with `redis-cli` and any standard Redis client library. It is built on [Tokio](https://tokio.rs/) and uses a **16-shard concurrent store** so multiple clients can operate in parallel without contending on a single global lock.

---

## Features

| Area | Details |
|---|---|
| **Protocol** | Full RESP — compatible with `redis-cli` and all Redis clients |
| **Data types** | Strings, Lists (`VecDeque`), Hashes (`HashMap`) |
| **Concurrency** | 16-shard internal store; each shard has its own `std::sync::RwLock` |
| **TTL / Expiry** | Millisecond precision (`EXPIRE`, `PEXPIRE`, `TTL`, `PTTL`, `PERSIST`) |
| **LRU eviction** | Least-recently-used key evicted per shard when the key limit is reached |
| **Transactions** | Atomic batching with `MULTI` / `EXEC` / `DISCARD` |
| **Authentication** | `AUTH` password (configurable via CLI; disable by passing `--password ""`) |
| **Persistence** | Atomic JSON snapshot every N seconds (write to `.tmp`, then rename) |
| **CLI config** | Port, host, password, max-keys, snapshot file, save interval — all via flags |

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) stable (≥ 1.75)

### Build & Run

```bash
git clone https://github.com/your-username/minikv.git
cd minikv
cargo run --release
```

Default startup output:

```
╔════════════════════════════════════════╗
║           mini_kv  v0.2.0              ║
║  Redis-compatible key-value server     ║
╚════════════════════════════════════════╝
  Listening on  : 127.0.0.1:6379
  Auth          : enabled
  Max keys      : 10000
  Snapshot file : data.json
  Save interval : 10s

[mini_kv] Ready to accept connections.
```

### CLI Options

```
Usage: mini_kv [OPTIONS]

Options:
      --host <HOST>                  [default: 127.0.0.1]
  -p, --port <PORT>                  [default: 6379]
      --password <PASSWORD>          [default: secret]
      --max-keys <MAX_KEYS>          [default: 10000]
      --db-file <DB_FILE>            [default: data.json]
      --save-interval <SAVE_INTERVAL>[default: 10]
  -h, --help
  -V, --version
```

Example — custom port, no auth, 1M key limit:

```bash
cargo run --release -- --port 7379 --password "" --max-keys 1000000
```

### Connect with redis-cli

```bash
redis-cli -p 6379
127.0.0.1:6379> AUTH secret
OK
127.0.0.1:6379> SET hello "world" EX 60
OK
127.0.0.1:6379> GET hello
"world"
127.0.0.1:6379> TTL hello
(integer) 59
```

---

## Supported Commands

### Connection

| Command | Description |
|---|---|
| `AUTH <password>` | Authenticate |
| `PING [message]` | Returns `PONG` or echoes message |
| `QUIT` | Close the connection |
| `SELECT 0` | Stub — only database 0 is supported |

### Strings

| Command | Description |
|---|---|
| `SET <key> <value> [EX s] [PX ms] [NX]` | Set a value with optional expiry / NX flag |
| `SETEX <key> <seconds> <value>` | Set with second-precision expiry |
| `PSETEX <key> <ms> <value>` | Set with millisecond-precision expiry |
| `SETNX <key> <value>` | Set only if key does not exist |
| `GET <key>` | Get a value |
| `GETSET <key> <value>` | Atomically set and return old value |
| `MSET <k> <v> [k v …]` | Set multiple keys at once |
| `MGET <k> [k …]` | Get multiple values at once |
| `INCR <key>` | Increment integer by 1 |
| `DECR <key>` | Decrement integer by 1 |
| `INCRBY <key> <n>` | Increment integer by n |
| `DECRBY <key> <n>` | Decrement integer by n |
| `APPEND <key> <value>` | Append to a string; returns new length |

### Generic

| Command | Description |
|---|---|
| `DEL <key> [key …]` | Delete one or more keys |
| `EXISTS <key>` | Check if key exists (`1` / `0`) |
| `TYPE <key>` | Returns `string`, `list`, `hash`, or `none` |
| `RENAME <src> <dst>` | Rename a key |
| `EXPIRE <key> <seconds>` | Set TTL in seconds |
| `PEXPIRE <key> <ms>` | Set TTL in milliseconds |
| `TTL <key>` | Remaining TTL in seconds (`-1` no expiry, `-2` missing) |
| `PTTL <key>` | Remaining TTL in milliseconds |
| `PERSIST <key>` | Remove TTL from a key |
| `KEYS <pattern>` | List keys matching a glob pattern (`*`, `?`) |
| `DBSIZE` | Number of live keys |
| `FLUSHALL` / `FLUSHDB` | Delete all keys |

### Hashes

| Command | Description |
|---|---|
| `HSET <key> <f> <v> [f v …]` | Set one or more fields |
| `HGET <key> <field>` | Get a field value |
| `HDEL <key> <field>` | Delete a field |
| `HGETALL <key>` | Get all field-value pairs |
| `HKEYS <key>` | Get all field names |
| `HVALS <key>` | Get all values |
| `HLEN <key>` | Number of fields |
| `HEXISTS <key> <field>` | Check if field exists |
| `HMGET <key> <f> [f …]` | Get multiple fields |
| `HMSET <key> <f> <v> [f v …]` | Set multiple fields (deprecated alias for HSET) |

### Lists

| Command | Description |
|---|---|
| `LPUSH <key> <value>` | Prepend a value |
| `RPUSH <key> <value>` | Append a value |
| `LPOP <key>` | Remove and return the first element |
| `RPOP <key>` | Remove and return the last element |
| `LLEN <key>` | List length |
| `LRANGE <key> <start> <stop>` | Get a slice (negative indices supported) |
| `LINDEX <key> <index>` | Get element at index |

### Transactions

| Command | Description |
|---|---|
| `MULTI` | Begin a transaction |
| `EXEC` | Execute all queued commands atomically |
| `DISCARD` | Discard all queued commands |

### Server

| Command | Description |
|---|---|
| `INFO` | Server stats (uptime, connections, commands, keyspace) |
| `DBSIZE` | Total live key count |
| `FLUSHALL` | Wipe all data |

---

## Architecture

```
main.rs
├── CLI (clap)  →  Config struct
├── TcpListener (port 6379)
├── tokio::spawn per client → handle_client()
│   ├── parse_command()     RESP + inline parser
│   ├── Session layer       AUTH / MULTI / EXEC / DISCARD / QUIT / INFO
│   └── execute_command()   delegates to KvStore (lock-free at call site)
└── Background task (every N seconds)
    ├── store.purge_expired()   active TTL sweep across all shards
    └── store.save()            atomic JSON snapshot

lib.rs  (KvStore)
├── 16 independent shards, each: std::sync::RwLock<Shard>
│   └── Shard: HashMap<String, Entry>
├── Entry: DataType + expires_at_ms (Option<i64>) + last_accessed_ms
├── DataType: String | List(VecDeque) | Hash(HashMap)
└── Utilities: glob_match(), normalize_index(), shard_idx()
```

### Why 16 shards?

The original design used a single `Arc<RwLock<KvStore>>`. Because LRU needs to update `last_accessed` even on reads, every `GET` required a **write lock**, serialising all clients. With 16 shards, concurrent operations on different key-spaces proceed in parallel — a ~16× throughput improvement under realistic workloads.

---

## Persistence

Every N seconds (configurable, default 10 s), a background task:
1. **Sweeps** all shards to remove expired keys.
2. **Snapshots** the live store to a `.tmp` file, then **atomically renames** it to the final path, so a crash mid-write never corrupts the on-disk data.

On startup, the snapshot is loaded and any already-expired keys are silently dropped.

---

## Error Handling

- `WRONGTYPE` — returned when a command is used against the wrong data type (e.g. `GET` on a hash), matching Redis semantics exactly.
- `NotInteger` / `Overflow` — returned for `INCR`/`DECR` on non-integer or overflowing values.
- `InvalidExpiry` — returned when a TTL ≤ 0 is supplied to `SET EX` / `SETEX` / `PEXPIRE`.

---

## Dependencies

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime, TCP I/O, background tasks |
| `serde` + `serde_json` | JSON snapshot serialization |
| `clap` | CLI argument parsing |

---

## Running the Test Suite

```bash
cargo build
python3 test_smoke.py   # 65 RESP-level integration tests
```

---

## License

MIT
