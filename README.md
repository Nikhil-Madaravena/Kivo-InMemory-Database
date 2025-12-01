# 🗄️ MiniKV — A Lightweight In-Memory Database in Rust

A fast, Redis-like **in-memory key–value database** written in Rust from scratch.
Supports **strings, lists, counters, TTL**, JSON-based persistence, and a simple TCP text protocol.

Built to learn systems programming, storage engines, and database internals.

---

## ✨ Features

### ✔ Core Database

* In-memory `HashMap` engine
* String values (`SET`, `GET`, `DEL`, `EXISTS`)
* Auto-cleanup of expired keys (manual TTL)
* Atomic counters (`INCR`, `DECR`)
* JSON persistence (`data.json`)

### ✔ Redis-Style Lists

* `LPUSH` — push values to the head
* `RPUSH` — push values to the tail
* `LPOP` — pop from head
* `RPOP` — pop from tail
* `LRANGE` — slice lists with positive/negative indices

### ✔ TTL

* `EXPIRE key seconds`
* `TTL key`

### ✔ Server

* Multi-threaded TCP server
* Simple, human-readable text protocol
* Thread-safe (`Arc<Mutex<T>>`)
* Runs on port **6379** (same as Redis)

---

## 🚀 Getting Started

### 1️⃣ Clone the repo

```bash
git clone https://github.com/yourusername/minikv.git
cd minikv
```

### 2️⃣ Build & run

```bash
cargo run
```

You should see:

```
mini_kv server listening on 127.0.0.1:6379
```

---

## 🧪 Using the Database

You interact with MiniKV via **netcat**:

```bash
nc 127.0.0.1 6379
```

### 🔤 String commands

```
SET name Nikhil
GET name
EXPIRE name 5
TTL name
DEL name
```

### 🔢 Counter commands

```
INCR visits
INCR visits
DECR visits
```

### 📝 List commands

```
LPUSH mylist 10
RPUSH mylist 20
LRANGE mylist 0 -1
LPOP mylist
RPOP mylist
```

### 🧹 Other commands

```
EXISTS key
FLUSHALL
QUIT
```

---

## 📁 Persistence

MiniKV stores all data in `data.json`:

* On every write, the file is safely rewritten (atomic write).
* On startup, MiniKV loads the file into memory.

Future planned:
✔ Append-Only File (AOF)
✔ RDB snapshotting

---

## 🏗 Project Structure

```
mini_kv/
├── src/
│   ├── lib.rs      # KvStore engine
│   ├── main.rs     # TCP server
├── data.json       # Persistence file
├── Cargo.toml
└── README.md
```

---

## 🧠 Internals (How MiniKV Works)

### Storage Engine

* In-memory `HashMap<String, Entry>`
* `Entry` contains:

  * `Value` enum (`String` or `List`)
  * optional TTL timestamp

### Concurrency

* `Arc<Mutex<KvStore>>` shared across worker threads
* Each client handled in its own thread

### Persistence

* Entire DB written as pretty JSON
* Temporary file → atomic rename for safety

---

## 🛣 Roadmap

### 🚀 Coming next

* RESP protocol support (use `redis-cli` with MiniKV)
* Lists: LLEN, LINDEX, LSET
* Sets & Hashes
* async I/O with Tokio
* TTL cleanup thread
* AOF (append-only log)
* Background snapshotting
* Cluster mode / replication
* Web dashboard (React + WebSockets)

If you want any of these now, open an issue or PR!

---

## 🤝 Contributing

Contributions are welcome!
You can help with:

* New commands
* Optimizations
* Bug fixes
* Documentation improvements
* Adding tests
* Adding benchmarks

---

## 📝 License

MIT License — feel free to use MiniKV in your projects.

---

## ⭐ Like this project?

Star the repo to support development!
It motivates more features: clustering, RESP support, sharded maps, async runtime, and more 🚀

