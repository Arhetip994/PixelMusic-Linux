use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const MAX_HEAD: usize = 64 * 1024;
const MAX_BODY: usize = 8 * 1024 * 1024;

pub fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' => {
                let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok());
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(b) => {
                        out.push(b);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn parse_query(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| match p.split_once('=') {
            Some((k, v)) => (decode(k), decode(v)),
            None => (decode(p), String::new()),
        })
        .collect()
}

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub body: String,
}

impl Request {
    pub fn param(&self, key: &str) -> Option<&str> {
        self.query.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
        .or_else(|| buf.windows(2).position(|w| w == b"\n\n").map(|p| p + 2))
}

pub async fn read_request(stream: &mut TcpStream, limit: Duration) -> Option<Request> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = timeout(limit, async {
        loop {
            if let Some(end) = find_head_end(&buf) {
                return Some(end);
            }
            if buf.len() > MAX_HEAD {
                return None;
            }
            let n = stream.read(&mut chunk).await.ok()?;
            if n == 0 {
                return find_head_end(&buf).or((!buf.is_empty()).then_some(buf.len()));
            }
            buf.extend_from_slice(&chunk[..n]);
        }
    }).await.ok()??;

    let head = std::str::from_utf8(&buf[..head_end]).ok()?;
    let mut lines = head.lines();
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_uppercase();
    let full_path = parts.next()?;

    let (path, raw_query) = match full_path.split_once('?') {
        Some((p, q)) => (p.to_string(), q),
        None => (full_path.to_string(), ""),
    };
    let query = parse_query(raw_query);

    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body = buf[head_end..].to_vec();
    if content_length > 0 && body.len() < content_length && content_length <= MAX_BODY {
        let needed = content_length - body.len();
        let mut rest = vec![0u8; needed];
        if timeout(limit, stream.read_exact(&mut rest)).await.ok()?.is_ok() {
            body.extend_from_slice(&rest);
        }
    }

    Some(Request {
        method,
        path,
        query,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

pub async fn respond_json(stream: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await
}

pub async fn send_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: &str,
    timeout_duration: Duration,
) -> anyhow::Result<(u16, String)> {
    let mut stream = timeout(timeout_duration, TcpStream::connect(addr)).await??;
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).await?;
    stream.flush().await?;

    let mut response_buf = Vec::new();
    timeout(timeout_duration, stream.read_to_end(&mut response_buf)).await??;

    let res_text = String::from_utf8_lossy(&response_buf);
    let mut status_code = 200u16;
    if let Some(first_line) = res_text.lines().next() {
        let mut parts = first_line.split_whitespace();
        let _http_ver = parts.next();
        if let Some(code_str) = parts.next() {
            status_code = code_str.parse().unwrap_or(200);
        }
    }

    let body_text = if let Some(idx) = res_text.find("\r\n\r\n") {
        &res_text[idx + 4..]
    } else if let Some(idx) = res_text.find("\n\n") {
        &res_text[idx + 2..]
    } else {
        ""
    };

    Ok((status_code, body_text.to_string()))
}
