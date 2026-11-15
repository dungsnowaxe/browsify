//! Exercise the real CLI/HTTP boundary with an isolated HOME and asset directory.
use browsify_core::RulesFile;
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static NEXT_SERVER: AtomicUsize = AtomicUsize::new(0);
// UI processes share a per-user singleton lock even with distinct HOME values.
// Serialize setup as well to avoid copying executables while another test spawns.
static SERVER_LOCK: Mutex<()> = Mutex::new(());

struct Server {
    _guard: MutexGuard<'static, ()>,
    child: Child,
    root: PathBuf,
    addr: String,
    token: String,
    default_rules: PathBuf,
    rules_path: PathBuf,
}

impl Server {
    fn start(custom_rules: bool) -> Self {
        let guard = SERVER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let root = std::env::temp_dir().join(format!(
            "browsify-http-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_SERVER.fetch_add(1, Ordering::Relaxed)
        ));
        let home = root.join("home");
        let dist = root.join("apps/browsify-ui/dist");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(dist.join("assets")).unwrap();
        fs::write(
            dist.join("index.html"),
            "<head></head><body>UI fixture</body>",
        )
        .unwrap();
        fs::write(dist.join("assets/app.js"), "// public UI asset").unwrap();
        fs::write(
            dist.parent().unwrap().join("marker.txt"),
            "outside-asset-marker",
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            dist.parent().unwrap().join("marker.txt"),
            dist.join("assets/escape.txt"),
        )
        .unwrap();
        let default_rules = RulesFile::default_path(&home);
        fs::create_dir_all(default_rules.parent().unwrap()).unwrap();
        let options = json!({"hideIcon": true, "logRequests": true, "keepRunning": false, "checkForUpdates": false});
        fs::write(
            &default_rules,
            json!({"defaultBrowser": "Safari", "options": options, "rules": []}).to_string(),
        )
        .unwrap();
        let rules_path = if custom_rules {
            let path = root.join("custom/rules.json");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(
                &path,
                json!({"defaultBrowser": "Google Chrome", "options": options, "rules": []})
                    .to_string(),
            )
            .unwrap();
            path
        } else {
            default_rules.clone()
        };
        // Copy the executable so its asset discovery cannot use a developer's build output.
        let bin = root.join("browsify");
        fs::copy(env!("CARGO_BIN_EXE_browsify"), &bin).unwrap();
        let mut command = Command::new(bin);
        command
            .current_dir(&root)
            .env("HOME", &home)
            .args(["ui", "--no-config", "--no-open"])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if custom_rules {
            command.arg("--rules").arg(&rules_path);
        }
        let mut child = command.spawn().unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let url = line
            .trim()
            .strip_prefix("Browsify UI listening on http://")
            .expect("server startup URL");
        let (addr, token) = url.split_once("/?token=").unwrap();
        Self {
            _guard: guard,
            child,
            root,
            addr: addr.into(),
            token: token.into(),
            default_rules,
            rules_path,
        }
    }

    fn request(&self, chunks: &[&[u8]]) -> (u16, String) {
        let (status, body, _) = self.request_with_headers(chunks);
        (status, body)
    }

    fn request_with_headers(&self, chunks: &[&[u8]]) -> (u16, String, String) {
        let mut stream = TcpStream::connect(&self.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(8)))
            .unwrap();
        stream.set_nodelay(true).unwrap();
        for (i, chunk) in chunks.iter().enumerate() {
            if i > 0 {
                thread::sleep(Duration::from_millis(40));
            }
            if stream.write_all(chunk).is_err() {
                break;
            }
        }
        let _ = stream.shutdown(Shutdown::Write);
        let mut response = Vec::new();
        let mut buf = [0; 8192];
        // A rejected request may close with unread bytes, causing a reset.
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
            response.extend_from_slice(&buf[..n]);
        }
        let response = String::from_utf8(response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap_or_else(|| {
            panic!(
                "Missing HTTP response for {:?}",
                String::from_utf8_lossy(&chunks[0][..chunks[0].len().min(200)])
            )
        });
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, body.into(), head.into())
    }

    fn get(&self, target: &str) -> (u16, String) {
        self.request(&[format!("GET {target} HTTP/1.1\r\nHost: {}\r\n\r\n", self.addr).as_bytes()])
    }

    fn rpc_headers(&self, size: usize) -> String {
        format!("POST /api/rpc HTTP/1.1\r\nHost: {}\r\nX-Browsify-Token: {}\r\nContent-Length: {size}\r\n\r\n", self.addr, self.token)
    }

    fn rpc(&self, message: Value) -> Value {
        let body = message.to_string();
        let request = format!("{}{}", self.rpc_headers(body.len()), body);
        let (status, response) = self.request(&[request.as_bytes()]);
        assert_eq!(status, 200, "{response}");
        serde_json::from_str(&response).unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn static_files_cannot_escape_asset_directory() {
    let server = Server::start(false);
    for path in [
        "/../marker.txt",
        "/assets/../../marker.txt",
        "/%2e%2e/marker.txt",
        "/assets/escape.txt",
    ] {
        let (status, body) = server.get(path);
        assert!(status == 403 || status == 404, "{path}: {status}: {body}");
        assert!(!body.contains("outside-asset-marker"));
    }
    assert_eq!(
        server.get("/assets/app.js"),
        (200, "// public UI asset".into())
    );
}

#[test]
fn only_authenticated_documents_receive_the_session_token() {
    let server = Server::start(false);
    for path in ["/", "/rules", "/index.html", "/missing.js"] {
        let (status, body) = server.get(path);
        assert_ne!(status, 200, "{path} must not expose a usable UI session");
        assert!(!body.contains(&server.token));
    }
    for path in ["/", "/rules", "/index.html"] {
        let (status, body) = server.get(&format!("{path}?token={}", server.token));
        assert_eq!(status, 200);
        assert!(body.contains("UI fixture"));
        assert!(body.contains(&server.token));
    }
    // Follow the cookie actually issued at bootstrap, as a browser refresh does
    // after client-side navigation has removed the query string.
    let request = format!(
        "GET /?token={} HTTP/1.1\r\nHost: {}\r\n\r\n",
        server.token, server.addr
    );
    let (_, _, headers) = server.request_with_headers(&[request.as_bytes()]);
    let cookie = headers
        .lines()
        .find_map(|line| line.strip_prefix("Set-Cookie: "))
        .unwrap();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    let request = format!(
        "GET /rules HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\n\r\n",
        server.addr,
        cookie.split(';').next().unwrap()
    );
    assert_eq!(server.request(&[request.as_bytes()]).0, 200);
    assert_eq!(server.get("/api/events").0, 403);
}

#[test]
fn rpc_reads_fragmented_headers_and_bodies_including_large_utf8_payloads() {
    let server = Server::start(false);
    let body = json!({"type": "getRules", "padding": "界".repeat(24000)}).to_string();
    let headers = server.rpc_headers(body.len());
    let unicode = body.find('界').unwrap();
    // First deliver complete headers before any body; then also split the
    // request line/header terminator. Neither boundary is guaranteed by TCP.
    for header_chunks in [
        vec![headers.as_bytes()],
        vec![
            &headers.as_bytes()[..12],
            &headers.as_bytes()[12..headers.len() - 1],
            &headers.as_bytes()[headers.len() - 1..],
        ],
    ] {
        let mut chunks = header_chunks;
        chunks.extend([
            &body.as_bytes()[..unicode + 1],
            &body.as_bytes()[unicode + 1..],
        ]);
        let (status, response) = server.request(&chunks);
        assert_eq!(status, 200);
        let response: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(response["type"], "rules");
        assert_eq!(response["message"]["defaultBrowser"], "Safari");
    }
}

#[test]
fn malformed_or_oversized_requests_do_not_mutate_rules() {
    let server = Server::start(false);
    let before = fs::read(&server.rules_path).unwrap();
    let body =
        json!({"type": "saveRules", "payload": {"defaultBrowser": "Google Chrome", "rules": []}})
            .to_string();
    let truncated = format!("{}{}", server.rpc_headers(body.len() + 10), body);
    assert_eq!(server.request(&[truncated.as_bytes()]).0, 400);
    for extra in ["Content-Length: 1\r\n", "Transfer-Encoding: chunked\r\n"] {
        let headers = server.rpc_headers(body.len());
        let request = format!("{}\r\n{extra}\r\n{body}", headers.trim_end_matches("\r\n"));
        assert_eq!(server.request(&[request.as_bytes()]).0, 400);
    }
    assert_eq!(
        server
            .request(&[server.rpc_headers(1024 * 1024 + 1).as_bytes()])
            .0,
        413
    );
    let huge_header = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nX-Padding: {}\r\n\r\n",
        server.addr,
        "a".repeat(17000)
    );
    // Reach the limit without an end-of-headers marker. No more input is
    // necessary for the server to reject this request.
    assert_eq!(
        server.request(&[&huge_header.as_bytes()[..16 * 1024]]).0,
        413
    );
    assert_eq!(fs::read(&server.rules_path).unwrap(), before);
}

#[test]
fn saving_rules_without_options_preserves_preferences_but_explicit_changes_work() {
    let server = Server::start(false);
    let original = server.rpc(json!({"type": "getRules"}))["message"]["options"].clone();
    let response = server.rpc(json!({"type": "saveRules", "payload": {"defaultBrowser": "Safari", "rules": [{"match": "*example.com/*", "browser": "Google Chrome"}]}}));
    assert_eq!(response[0]["message"]["options"], original);
    let saved: Value = serde_json::from_slice(&fs::read(&server.rules_path).unwrap()).unwrap();
    assert_eq!(saved["options"], original);
    assert_eq!(saved["rules"][0]["browser"], "Google Chrome");
    let response = server.rpc(json!({"type": "saveRules", "payload": {"defaultBrowser": "Safari", "options": {"hideIcon": false, "logRequests": false, "checkForUpdates": false}, "rules": []}}));
    assert_eq!(response[1]["message"]["options"]["hideIcon"], false);
    assert_eq!(response[1]["message"]["options"]["logRequests"], false);
}

#[test]
fn custom_rules_path_is_used_for_reads_writes_and_routing_after_restart() {
    let server = Server::start(true);
    let default_before = fs::read(&server.default_rules).unwrap();
    let loaded = server.rpc(json!({"type": "getRules"}));
    let response = server.rpc(json!({"type": "saveRules", "payload": {"defaultBrowser": "Microsoft Edge", "options": {"checkForUpdates": false}, "rules": []}}));
    let saved: Value = serde_json::from_slice(&fs::read(&server.rules_path).unwrap()).unwrap();
    assert_eq!(saved["defaultBrowser"], "Microsoft Edge");
    assert_eq!(fs::read(&server.default_rules).unwrap(), default_before);
    assert_eq!(loaded["message"]["defaultBrowser"], "Google Chrome");
    assert_eq!(
        loaded["message"]["path"],
        server.rules_path.to_str().unwrap()
    );
    assert_eq!(
        response[0]["message"]["path"],
        server.rules_path.to_str().unwrap()
    );
    let output = Command::new(server.root.join("browsify"))
        .env("HOME", server.root.join("home"))
        .args(["test", "https://example.com/", "--no-config", "--rules"])
        .arg(&server.rules_path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Microsoft Edge"));
}
