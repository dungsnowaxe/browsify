use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use url::Url;

const MAX_SHORT_URL_CACHE_ENTRIES: usize = 256;

static SHORT_URL_CACHE: OnceLock<Mutex<VecDeque<(String, String)>>> = OnceLock::new();

pub const SHORTENER_DOMAINS: &[&str] = &[
    "adf.ly",
    "b.link",
    "bit.do",
    "bit.ly",
    "bl.ink",
    "buff.ly",
    "cutt.ly",
    "ClickMeter.com",
    "db.tt",
    "dub.co",
    "dub.sh",
    "goo.gl",
    "is.gd",
    "ity.im",
    "j.mp",
    "kutt.it",
    "lnkd.in",
    "ow.ly",
    "po.st",
    "qlink.me",
    "rb.gy",
    "rebrand.ly",
    "short.cm",
    "short.io",
    "shorturl.at",
    "s.id",
    "t.co",
    "t.ly",
    "tiny.cc",
    "tiny.pl",
    "tinyurl.com",
    "tr.im",
    "u.to",
    "v.gd",
    "vzturl.com",
    "x.co",
    "yourls.org",
    "zpr.io",
];

fn cache_lookup(url: &str) -> Option<String> {
    let cache = SHORT_URL_CACHE.get_or_init(|| Mutex::new(VecDeque::new()));
    let mut guard = cache.lock().ok()?;
    if let Some(idx) = guard.iter().position(|(k, _)| k == url) {
        let pair = guard.remove(idx)?;
        let val = pair.1.clone();
        guard.push_front(pair);
        return Some(val);
    }
    None
}

pub fn cache_insert(url: String, resolved: String) {
    let cache = SHORT_URL_CACHE.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Ok(mut guard) = cache.lock() {
        if let Some(idx) = guard.iter().position(|(k, _)| k == &url) {
            guard.remove(idx);
        }
        guard.push_front((url, resolved));
        while guard.len() > MAX_SHORT_URL_CACHE_ENTRIES {
            guard.pop_back();
        }
    }
}

/// Checks whether `url_str` belongs to a known URL shortener domain (or any custom `extra_domains`).
pub fn is_short_url(url_str: &str, extra_domains: &[String]) -> bool {
    let Ok(parsed) = Url::parse(url_str) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let matches_domain = |domain: &str| {
        host.eq_ignore_ascii_case(domain)
            || (host.len() > domain.len()
                && host
                    .to_ascii_lowercase()
                    .ends_with(&format!(".{}", domain.to_ascii_lowercase())))
    };
    SHORTENER_DOMAINS.iter().any(|d| matches_domain(d))
        || extra_domains.iter().any(|d| matches_domain(d))
}

/// Resolves a shortened URL by consulting the 256-entry LRU cache first, then following
/// up to 3 plain-HTTP (`http://`) redirects in-process with a 750 ms per-hop timeout.
/// HTTPS shorteners are returned unresolved (a native `NSURLSession` HTTPS expander is
/// planned for macOS). Falls back to `original_url` on any network error or if the
/// domain is not a known shortener.
pub fn resolve_short_url(original_url: &str, extra_domains: &[String]) -> String {
    if !is_short_url(original_url, extra_domains) {
        return original_url.to_string();
    }

    if let Some(cached) = cache_lookup(original_url) {
        return cached;
    }

    // In-process plain HTTP redirect follower for `http://` URLs
    if let Some(resolved) = try_resolve_http_in_process(original_url) {
        if resolved != original_url {
            cache_insert(original_url.to_string(), resolved.clone());
            return resolved;
        }
    }

    original_url.to_string()
}

fn try_resolve_http_in_process(start_url: &str) -> Option<String> {
    let mut current = Url::parse(start_url).ok()?;
    let timeout = Duration::from_millis(750);

    for _ in 0..3 {
        if current.scheme() != "http" {
            return None;
        }
        let host = current.host_str()?;
        let port = current.port().unwrap_or(80);
        let addr = (host, port).to_socket_addrs().ok()?.next()?;
        let mut stream = TcpStream::connect_timeout(&addr, timeout).ok()?;
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));

        let path_and_query = match current.query() {
            Some(q) => format!("{}?{}", current.path(), q),
            None => current.path().to_string(),
        };
        let req = format!(
            "HEAD {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Browsify/0.1\r\nConnection: close\r\n\r\n",
            path_and_query, host
        );
        stream.write_all(req.as_bytes()).ok()?;

        let headers = read_http_headers(&mut stream, 16384)?;
        let mut lines = headers.lines();
        let status_line = lines.next()?;
        let status_code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())?;

        let mut location = None;
        for line in lines {
            if line.is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                if k.trim().eq_ignore_ascii_case("location") {
                    location = Some(v.trim().to_string());
                    break;
                }
            }
        }
        if let Some(loc) = location {
            current = current.join(&loc).ok()?;
        } else if (200..400).contains(&status_code) {
            break;
        } else {
            return None;
        }
    }
    Some(current.into())
}

fn read_http_headers(stream: &mut TcpStream, max_bytes: usize) -> Option<String> {
    let mut acc = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while acc.len() < max_bytes {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        acc.extend_from_slice(&chunk[..n]);
        if let Some(end_idx) = find_header_end(&acc) {
            return Some(String::from_utf8_lossy(&acc[..end_idx]).into_owned());
        }
    }
    None
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        return Some(pos + 4);
    }
    if let Some(pos) = buf.windows(2).position(|w| w == b"\n\n") {
        return Some(pos + 2);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_short_url_detection_and_lru_cache() {
        assert!(is_short_url("https://t.co/abc123", &[]));
        assert!(is_short_url("https://bit.ly/xyz", &[]));
        assert!(is_short_url(
            "https://go.internal.corp/link",
            &["go.internal.corp".to_string()]
        ));
        assert!(!is_short_url("https://github.com/snowaxe/browsify", &[]));

        cache_insert(
            "https://t.co/cached123".to_string(),
            "https://github.com/snowaxe/browsify".to_string(),
        );
        assert_eq!(
            resolve_short_url("https://t.co/cached123", &[]),
            "https://github.com/snowaxe/browsify"
        );
    }

    #[test]
    fn test_multi_chunk_http_redirect_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            // Hop 1: sends status line and Location header in separate TCP writes
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 512];
                let _ = stream.read(&mut req_buf);
                let _ = stream.write_all(b"HTTP/1.1 302 Found\r\nServer: test\r\n");
                let _ = stream.flush();
                thread::sleep(Duration::from_millis(15));
                let second_chunk = format!(
                    "Location: http://{}/final-destination\r\n\r\n",
                    addr
                );
                let _ = stream.write_all(second_chunk.as_bytes());
                let _ = stream.flush();
            }
            // Hop 2: final 200 OK
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 512];
                let _ = stream.read(&mut req_buf);
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
                let _ = stream.flush();
            }
        });

        let start = format!("http://{}/short", addr);
        let resolved = try_resolve_http_in_process(&start).unwrap();
        assert_eq!(resolved, format!("http://{}/final-destination", addr));
        let _ = handle.join();
    }
}
