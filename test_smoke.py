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

    # --- List extras ---
    cmd(s, "FLUSHALL")
    cmd(s, "RPUSH", "lst", "a")
    cmd(s, "RPUSH", "lst", "b")
    cmd(s, "RPUSH", "lst", "c")
    cmd(s, "RPUSH", "lst", "b")
    cmd(s, "RPUSH", "lst", "b")

    chk("LSET",    cmd(s, "LSET", "lst", "0", "X"),          "OK")
    chk("LSET get",cmd(s, "LINDEX", "lst", "0"),             "X")
    chk("LREM 2",  cmd(s, "LREM", "lst", "2", "b"),          2)
    chk("LREM len",cmd(s, "LLEN", "lst"),                    3)   # X c b
    chk("LTRIM",   cmd(s, "LTRIM", "lst", "1", "2"),         "OK")
    chk("LTRIM len",cmd(s, "LLEN", "lst"),                   2)

    # --- Sets ---
    cmd(s, "FLUSHALL")
    chk("SADD",        cmd(s, "SADD", "s", "a", "b", "c"),  3)
    chk("SADD dup",    cmd(s, "SADD", "s", "a"),             0)
    chk("SCARD",       cmd(s, "SCARD", "s"),                 3)
    chk("SISMEMBER y", cmd(s, "SISMEMBER", "s", "a"),        1)
    chk("SISMEMBER n", cmd(s, "SISMEMBER", "s", "z"),        0)
    chk("SMEMBERS",    sorted(cmd(s, "SMEMBERS", "s")),      ["a","b","c"])
    chk("SREM",        cmd(s, "SREM", "s", "a", "b"),        2)
    chk("SCARD after", cmd(s, "SCARD", "s"),                 1)

    cmd(s, "SADD", "s1", "a", "b", "c")
    cmd(s, "SADD", "s2", "b", "c", "d")
    chk("SUNION",  sorted(cmd(s, "SUNION", "s1", "s2")),  ["a","b","c","d"])
    chk("SINTER",  sorted(cmd(s, "SINTER", "s1", "s2")),  ["b","c"])
    chk("SDIFF",   sorted(cmd(s, "SDIFF",  "s1", "s2")),  ["a"])

    chk("SMOVE",   cmd(s, "SMOVE", "s1", "s2", "a"),         1)
    chk("SMOVE src",sorted(cmd(s, "SMEMBERS", "s1")),        ["b","c"])
    chk("SMOVE dst",sorted(cmd(s, "SMEMBERS", "s2")),        ["a","b","c","d"])

    chk("SUNIONSTORE", cmd(s, "SUNIONSTORE", "su", "s1", "s2"), 4)
    chk("SINTERSTORE", cmd(s, "SINTERSTORE", "si", "s1", "s2"), 2)
    chk("SDIFFSTORE",  cmd(s, "SDIFFSTORE",  "sd", "s2", "s1"), 2)  # d and a

    spop_result = cmd(s, "SPOP", "s1")
    chk("SPOP type", isinstance(spop_result, str), True)
    srand = cmd(s, "SRANDMEMBER", "s2")
    chk("SRANDMEMBER type", isinstance(srand, str), True)

    chk("TYPE set", cmd(s, "TYPE", "s1"), "set")

    # --- Sorted Sets ---
    cmd(s, "FLUSHALL")
    chk("ZADD",       cmd(s, "ZADD", "z", "1", "a", "2", "b", "3", "c"),  3)
    chk("ZADD dup",   cmd(s, "ZADD", "z", "5", "a"),                       0)  # update, not new
    chk("ZCARD",      cmd(s, "ZCARD", "z"),                                 3)
    chk("ZSCORE",     cmd(s, "ZSCORE", "z", "a"),                          "5")
    chk("ZSCORE nil", cmd(s, "ZSCORE", "z", "nope"),                        None)
    chk("ZRANK a",    cmd(s, "ZRANK", "z", "b"),                            0)  # b=2 is lowest
    chk("ZREVRANK a", cmd(s, "ZREVRANK", "z", "b"),                         2)
    chk("ZINCRBY",    cmd(s, "ZINCRBY", "z", "10", "b"),                   "12")
    chk("ZCOUNT",     cmd(s, "ZCOUNT", "z", "1", "10"),                     2)  # c=3, a=5
    chk("ZRANGE",     cmd(s, "ZRANGE", "z", "0", "-1"),                    ["c","a","b"])
    chk("ZREVRANGE",  cmd(s, "ZREVRANGE", "z", "0", "-1"),                 ["b","a","c"])
    chk("ZRANGEBYSCORE", cmd(s, "ZRANGEBYSCORE", "z", "1", "6"),           ["c","a"])
    chk("ZREM",       cmd(s, "ZREM", "z", "a", "b"),                        2)
    chk("ZCARD after",cmd(s, "ZCARD", "z"),                                 1)

    cmd(s, "ZADD", "zpop", "1", "x", "2", "y", "3", "z")
    chk("ZPOPMIN",    cmd(s, "ZPOPMIN", "zpop"),   ["x", "1"])
    chk("ZPOPMAX",    cmd(s, "ZPOPMAX", "zpop"),   ["z", "3"])

    chk("TYPE zset",  cmd(s, "TYPE", "z"), "zset")

    # --- SCAN ---
    cmd(s, "FLUSHALL")
    cmd(s, "SET", "k1", "v")
    cmd(s, "SET", "k2", "v")
    cmd(s, "SADD", "s_set", "x")
    scan_result = cmd(s, "SCAN", "0")
    chk("SCAN cursor", scan_result[0], "0")
    chk("SCAN count",  len(scan_result[1]), 3)
    scan_match = cmd(s, "SCAN", "0", "MATCH", "k*")
    chk("SCAN MATCH",  sorted(scan_match[1]), ["k1","k2"])
    scan_type = cmd(s, "SCAN", "0", "TYPE", "set")
    chk("SCAN TYPE",   scan_type[1], ["s_set"])

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

