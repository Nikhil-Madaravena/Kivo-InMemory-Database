#!/usr/bin/env python3
"""
Smoke-test mini_kv using raw RESP sockets.
Starts the server, runs assertions, shuts it down.
"""
import socket, subprocess, time, sys, os, signal

PORT = 16399  # use a non-standard port to avoid conflict

def send_cmd(sock, *args):
    msg = f"*{len(args)}\r\n"
    for a in args:
        a = str(a)
        msg += f"${len(a)}\r\n{a}\r\n"
    sock.sendall(msg.encode())

def recv_line(sock):
    buf = b""
    while not buf.endswith(b"\r\n"):
        buf += sock.recv(1)
    return buf.decode().strip()

def recv_resp(sock):
    line = recv_line(sock)
    t = line[0]
    data = line[1:]
    if t == '+': return data
    if t == '-': return Exception(data)
    if t == ':': return int(data)
    if t == '$':
        n = int(data)
        if n == -1: return None
        payload = b""
        while len(payload) < n + 2:
            payload += sock.recv(n + 2 - len(payload))
        return payload[:-2].decode()
    if t == '*':
        n = int(data)
        return [recv_resp(sock) for _ in range(n)]
    raise ValueError(f"Unknown RESP type: {t!r} in {line!r}")

def cmd(sock, *args):
    send_cmd(sock, *args)
    return recv_resp(sock)

def assert_eq(label, got, want):
    # Compare Exceptions by their string representation
    g = str(got) if isinstance(got, Exception) else got
    w = str(want) if isinstance(want, Exception) else want
    if g != w:
        print(f"FAIL [{label}]: got={got!r} want={want!r}")
        return False
    print(f"PASS [{label}]")
    return True

# --- Start server ---
env = os.environ.copy()
proc = subprocess.Popen(
    ["./target/debug/mini_kv", "--port", str(PORT), "--password", "testpass"],
    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    env=env
)
time.sleep(0.5)

try:
    s = socket.create_connection(("127.0.0.1", PORT), timeout=3)

    failures = 0

    def chk(label, got, want):
        global failures
        if not assert_eq(label, got, want):
            failures += 1

    # Auth
    chk("NOAUTH",  cmd(s, "PING"),            Exception("NOAUTH Authentication required."))
    chk("AUTH bad",cmd(s, "AUTH", "wrong"),   Exception("WRONGPASS invalid username-password pair or user is disabled."))
    chk("AUTH ok", cmd(s, "AUTH", "testpass"),"OK")

    # PING
    chk("PING",    cmd(s, "PING"),            "PONG")
    chk("PING msg",cmd(s, "PING", "hello"),   "hello")

    # Strings
    chk("SET",     cmd(s, "SET", "k", "v"),   "OK")
    chk("GET",     cmd(s, "GET", "k"),        "v")
    chk("GET nil", cmd(s, "GET", "nope"),     None)

    # SET EX
    chk("SETEX",   cmd(s, "SET", "ex", "val", "EX", "100"), "OK")
    ttl = cmd(s, "TTL", "ex")
    chk("TTL>0",   ttl > 0,                   True)

    # INCR / DECR
    cmd(s, "SET", "n", "10")
    chk("INCR",    cmd(s, "INCR", "n"),       11)
    chk("INCRBY",  cmd(s, "INCRBY", "n", "5"),16)
    chk("DECR",    cmd(s, "DECR", "n"),       15)
    chk("DECRBY",  cmd(s, "DECRBY", "n", "5"),10)

    # APPEND
    cmd(s, "SET", "app", "hello")
    chk("APPEND",  cmd(s, "APPEND", "app", " world"), 11)
    chk("APPEND get", cmd(s, "GET", "app"),  "hello world")

    # SETNX
    chk("SETNX new",  cmd(s, "SETNX", "nx", "1"), 1)
    chk("SETNX exist",cmd(s, "SETNX", "nx", "2"), 0)
    chk("SETNX val",  cmd(s, "GET", "nx"),         "1")

    # MSET / MGET
    chk("MSET",    cmd(s, "MSET", "a", "1", "b", "2", "c", "3"), "OK")
    chk("MGET",    cmd(s, "MGET", "a", "b", "c", "nope"),  ["1","2","3",None])

    # GETSET
    chk("GETSET",  cmd(s, "GETSET", "a", "99"), "1")
    chk("GETSET v",cmd(s, "GET", "a"),          "99")

    # EXISTS / DEL / TYPE
    chk("EXISTS 1", cmd(s, "EXISTS", "a"),    1)
    chk("TYPE str", cmd(s, "TYPE", "a"),      "string")
    chk("DEL",      cmd(s, "DEL", "a", "b"), 2)
    chk("EXISTS 0", cmd(s, "EXISTS", "a"),    0)

    # EXPIRE / TTL / PERSIST
    cmd(s, "SET", "tmp", "x")
    chk("EXPIRE ok",  cmd(s, "EXPIRE", "tmp", "100"),  1)
    chk("TTL set",    cmd(s, "TTL", "tmp") > 0,        True)
    chk("PERSIST",    cmd(s, "PERSIST", "tmp"),         1)
    chk("TTL after persist", cmd(s, "TTL", "tmp"),     -1)

    # PEXPIRE / PTTL
    cmd(s, "SET", "pt", "x")
    chk("PEXPIRE",    cmd(s, "PEXPIRE", "pt", "100000"), 1)
    chk("PTTL >0",    cmd(s, "PTTL", "pt") > 0,          True)

    # RENAME
    cmd(s, "SET", "orig", "hello")
    chk("RENAME",  cmd(s, "RENAME", "orig", "dest"), "OK")
    chk("RENAME get", cmd(s, "GET", "dest"),          "hello")
    chk("RENAME src gone", cmd(s, "GET", "orig"),     None)

    # DBSIZE
    cmd(s, "FLUSHALL")
    cmd(s, "SET", "x1", "1")
    cmd(s, "SET", "x2", "2")
    chk("DBSIZE", cmd(s, "DBSIZE"), 2)

    # KEYS glob
    cmd(s, "FLUSHALL")
    cmd(s, "SET", "user:1", "a")
    cmd(s, "SET", "user:2", "b")
    cmd(s, "SET", "other",  "c")
    keys = sorted(cmd(s, "KEYS", "user:*"))
    chk("KEYS glob", keys, ["user:1", "user:2"])
    chk("KEYS all",  sorted(cmd(s, "KEYS", "*")), ["other","user:1","user:2"])
    chk("KEYS ?", sorted(cmd(s, "KEYS", "user:?")), ["user:1","user:2"])

    # Hashes
    cmd(s, "FLUSHALL")
    chk("HSET new",    cmd(s, "HSET", "h", "f1", "v1", "f2", "v2"), 2)
    chk("HGET",        cmd(s, "HGET", "h", "f1"),                    "v1")
    chk("HGET nil",    cmd(s, "HGET", "h", "nope"),                  None)
    chk("HLEN",        cmd(s, "HLEN", "h"),                          2)
    chk("HEXISTS yes", cmd(s, "HEXISTS", "h", "f1"),                 1)
    chk("HEXISTS no",  cmd(s, "HEXISTS", "h", "nope"),               0)
    chk("HDEL",        cmd(s, "HDEL", "h", "f1"),                    1)
    chk("HLEN after",  cmd(s, "HLEN", "h"),                          1)
    all_kv = cmd(s, "HGETALL", "h")
    chk("HGETALL",     all_kv,                                        ["f2","v2"])
    chk("HKEYS",       cmd(s, "HKEYS", "h"),                         ["f2"])
    chk("HVALS",       cmd(s, "HVALS", "h"),                         ["v2"])

    # HMGET
    cmd(s, "HSET", "h2", "a", "1", "b", "2")
    chk("HMGET", cmd(s, "HMGET", "h2", "a", "b", "c"), ["1","2",None])

    # Lists
    cmd(s, "FLUSHALL")
    chk("LPUSH",   cmd(s, "LPUSH", "lst", "b"),    1)
    chk("LPUSH 2", cmd(s, "LPUSH", "lst", "a"),    2)
    chk("RPUSH",   cmd(s, "RPUSH", "lst", "c"),    3)
    chk("LLEN",    cmd(s, "LLEN", "lst"),           3)
    chk("LRANGE",  cmd(s, "LRANGE", "lst", "0", "-1"), ["a","b","c"])
    chk("LRANGE s",cmd(s, "LRANGE", "lst", "1", "2"),  ["b","c"])
    chk("LINDEX 0",cmd(s, "LINDEX", "lst", "0"),    "a")
    chk("LINDEX -1",cmd(s, "LINDEX", "lst", "-1"),  "c")
    chk("LPOP",    cmd(s, "LPOP", "lst"),            "a")
    chk("RPOP",    cmd(s, "RPOP", "lst"),            "c")
    chk("LLEN a",  cmd(s, "LLEN", "lst"),            1)

    # WRONGTYPE
    cmd(s, "SET", "str", "hello")
    wt = cmd(s, "LPUSH", "str", "x")
    chk("WRONGTYPE", isinstance(wt, Exception) and "WRONGTYPE" in str(wt), True)

    # FLUSHALL
    cmd(s, "SET", "z", "1")
    cmd(s, "FLUSHALL")
    chk("FLUSHALL", cmd(s, "DBSIZE"), 0)

    s.close()
    print(f"\n{'='*40}")
    if failures == 0:
        print("ALL TESTS PASSED ✓")
    else:
        print(f"{failures} TEST(S) FAILED ✗")
        sys.exit(1)

finally:
    proc.terminate()
    proc.wait()
