# 🗄️ MiniKV — Lightweight In-Memory Database in Rust

A fast, Redis-inspired **in-memory key–value database**, built from scratch in Rust.
Supports **Strings, Lists, Sets, Counters, TTL**, and **JSON persistence**, exposed over a simple TCP server.

Perfect for learning **systems programming, databases, storage engines, and Rust concurrency**.

---

## ✨ Features

### 🧩 Core Data Types

- **String** values (`SET`, `GET`, `DEL`, `EXISTS`)
- **List** values (Redis-style `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LRANGE`)
- **Set** values (`SADD`, `SREM`, `SMEMBERS`, `SISMEMBER`)
- Safe type checks with detailed errors
- Atomic counters (`INCR`, `DECR`)
- Key expiration (`EXPIRE`, `TTL`)

### 🧠 Storage Engine

- In-memory storage backed by Rust’s `HashMap`
- JSON file persistence (`data.json`)
- Atomic rewrite: write → temp file → rename
- Thread-safe execution via `Arc<Mutex<KvStore>>`

### 🔌 TCP Server

- Multi-threaded (one thread per client)
- Simple human-readable text protocol
- Runs on **127.0.0.1:6379**
- Works with `nc`, custom clients, or scripts

---

## 🚀 Getting Started

### 1️⃣ Clone & build

```bash
git clone https://github.com/yourusername/minikv.git
cd minikv
cargo run
```

You should see:

```
mini_kv server listening on 127.0.0.1:6379
```

---

## 🧪 Interacting with MiniKV

Use **netcat**:

```bash
nc 127.0.0.1 6379
```

---

# 🔤 String Commands

```
SET name Nikhil
GET name
EXISTS name
DEL name
```

---

# 🔢 Counters (Atomic)

```
INCR visits
INCR visits
DECR visits
```

---

# ⏳ TTL (Expiration)

```
EXPIRE session 10
TTL session
```

---

# 📜 List Commands

```
LPUSH mylist 10
RPUSH mylist 20
LRANGE mylist 0 -1
LPOP mylist
RPOP mylist
```

---

# 🧮 Set Commands

```
SADD tags rust
SADD tags rust db cli
SMEMBERS tags
SISMEMBER tags rust
SREM tags db
SMEMBERS tags
```

---

## 🧱 Project Structure

```
mini_kv/
├── src/
│   ├── lib.rs        # Core KV engine (Strings, Lists, Sets, TTL, persistence)
│   ├── main.rs       # TCP server and command protocol
├── data.json         # Persistent storage file
├── Cargo.toml
└── README.md
```

---

## 🧠 How It Works

### Storage Model

```
HashMap<String, Entry>
Entry {
  value: Value,        // String, List, Set
  expires_at: Option<timestamp>
}
```

### Concurrency

- Shared DB: `Arc<Mutex<KvStore>>`
- Each client handled in its own thread

### Persistence

- `serde_json` used for human-readable persistence
- Safe writes using temp files & atomic replace

---

## 🛣 Roadmap

### ✔ Completed

- Strings
- Lists
- Sets
- TTL
- Counters
- JSON persistence
- Multithreaded TCP server

### 🔜 Coming Soon

- More Set commands (`SCARD`, `SPOP`, `SRANDMEMBER`)
- More List commands (`LLEN`, `LINDEX`, `LSET`, `LTRIM`)
- Hash data type (`HSET`, `HGET`, `HGETALL`)
- TTL cleanup background thread
- AOF persistence (append-only log)
- RESP protocol (use `redis-cli` directly)
- Async server (Tokio)
- Web dashboard (React + WebSockets)
- Sharded map for high concurrency

---

## 🤝 Contributing

Contributions are welcome!

You can help with:

- Adding new commands
- Performance improvements
- Noise-free logging
- RESP protocol support
- Better error messages
- Adding tests

---

## 📝 License

MIT License.
Use it anywhere freely.

---

## ⭐ Support the Project

If you like this project, consider **starring** the repo —
it motivates continued development!
