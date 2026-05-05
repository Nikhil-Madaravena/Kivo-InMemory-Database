use kivo::KvError;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use bytes::{Bytes, BytesMut};

pub fn resp_ok()                    -> &'static str  { "+OK\r\n" }
pub fn resp_pong()                  -> &'static str  { "+PONG\r\n" }
pub fn resp_queued()                -> &'static str  { "+QUEUED\r\n" }
pub fn resp_nil()                   -> &'static str  { "$-1\r\n" }
pub fn resp_empty_array()           -> &'static str  { "*0\r\n" }
pub fn resp_int(v: i64)             -> String        { format!(":{v}\r\n") }
pub fn resp_err(msg: &str)          -> String        { format!("-ERR {msg}\r\n") }
pub fn resp_simple(msg: &str)       -> String        { format!("+{msg}\r\n") }
pub fn resp_bulk(val: &str)         -> String        { format!("${}\r\n{val}\r\n", val.len()) }

pub fn resp_array(items: &[String]) -> String {
    let mut out = format!("*{}\r\n", items.len());
    for item in items { out.push_str(&resp_bulk(item)); }
    out
}

pub fn resp_opt_bulk(v: Option<String>) -> String {
    match v { Some(s) => resp_bulk(&s), None => resp_nil().to_string() }
}

pub fn resp_kv_result<T, F>(r: Result<T, KvError>, f: F) -> String
where F: FnOnce(T) -> String {
    match r { Ok(v) => f(v), Err(e) => e.to_resp() }
}

pub async fn parse_command<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
) -> Result<Vec<Bytes>, String> {
    let mut line = String::new();
    if reader.read_line(&mut line).await.map_err(|e| e.to_string())? == 0 {
        return Err("EOF".into());
    }
    let line = line.trim_end_matches(['\r', '\n']);
    if line.is_empty() { return Ok(vec![]); }

    if line.starts_with('*') {
        let n: usize = line[1..].parse().map_err(|_| "bad array length")?;
        let mut args = Vec::with_capacity(n);
        for _ in 0..n {
            let mut hdr = String::new();
            reader.read_line(&mut hdr).await.map_err(|e| e.to_string())?;
            let hdr = hdr.trim_end_matches(['\r', '\n']);
            if !hdr.starts_with('$') { return Err("expected bulk string".into()); }
            let len: usize = hdr[1..].parse().map_err(|_| "bad bulk len")?;
            let mut buf = BytesMut::zeroed(len + 2);
            reader.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
            args.push(buf.freeze().slice(0..len));
        }
        Ok(args)
    } else {
        Ok(line.split_whitespace().map(|s| Bytes::copy_from_slice(s.as_bytes())).collect())
    }
}
