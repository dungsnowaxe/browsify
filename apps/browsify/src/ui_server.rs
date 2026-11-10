use crate::config_loader::BrowsifyEngine;
use browsify_core::{ProfileResolver, RulesFile, BROWSERS_REGISTRY};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Serves the on-demand Browsify UI (`apps/browsify-ui/dist` if present, or an embedded
/// zero-dependency Rules & URL Tester SPA) over `127.0.0.1:<port>` with 0 MB idle RAM when not running.
pub fn run_ui_server(
    engine: Arc<Mutex<BrowsifyEngine>>,
    profile_resolver: Arc<ProfileResolver>,
    port: u16,
    open_browser: bool,
) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Failed to bind UI server on 127.0.0.1:{}: {}", port, e))?;
    let local_addr = listener.local_addr().map_err(|e| e.to_string())?;
    let url = format!("http://{}", local_addr);

    println!("Browsify UI listening on {}", url);

    if open_browser {
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&url).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        }
    }

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let mut buf = [0u8; 65536];
        let Ok(n) = stream.read(&mut buf) else {
            continue;
        };
        if n == 0 {
            continue;
        }
        let req = String::from_utf8_lossy(&buf[..n]);
        let mut lines = req.lines();
        let Some(first_line) = lines.next() else {
            continue;
        };
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let method = parts[0];
        let path = parts[1];

        if method == "POST" && path == "/api/rpc" {
            let body = req
                .split("\r\n\r\n")
                .nth(1)
                .or_else(|| req.split("\n\n").nth(1))
                .unwrap_or("");
            let response_json = handle_rpc(body, &engine, &profile_resolver);
            let _ = write_http_response(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                response_json.as_bytes(),
            );
            continue;
        }

        // Serve static files from `apps/browsify-ui/dist` if built, otherwise serve embedded SPA
        let clean_path = path.trim_start_matches('/');
        let dist_dir = Path::new("apps/browsify-ui/dist");
        if dist_dir.is_dir() {
            let target = if clean_path.is_empty() || !clean_path.contains('.') {
                dist_dir.join("index.html")
            } else {
                dist_dir.join(clean_path)
            };
            if let Ok(bytes) = fs::read(&target) {
                let mime = guess_mime(&target);
                let _ = write_http_response(&mut stream, "200 OK", mime, &bytes);
                continue;
            }
        }

        let _ = write_http_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            FALLBACK_SPA_HTML.as_bytes(),
        );
    }

    Ok(())
}

fn handle_rpc(
    body: &str,
    engine: &Arc<Mutex<BrowsifyEngine>>,
    profile_resolver: &Arc<ProfileResolver>,
) -> String {
    let Ok(msg) = serde_json::from_str::<serde_json::Value>(body) else {
        return "[]".to_string();
    };
    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match msg_type {
        "getInitState" => {
            let eng = engine.lock().unwrap();
            let sum = eng.summary();
            let cfg_path = sum
                .config_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            serde_json::json!([
                { "type": "version", "message": format!("v{}", env!("CARGO_PKG_VERSION")) },
                { "type": "buildInfo", "message": format!("({})", sum.format) },
                {
                    "type": "config",
                    "message": {
                        "handlers": sum.handlers_count,
                        "rewrites": sum.rewrites_count,
                        "defaultBrowser": sum.default_browser,
                        "configPath": cfg_path,
                        "isJSConfig": sum.format.starts_with("JS/TS"),
                        "options": {
                            "keepRunning": sum.options.keep_running,
                            "hideIcon": sum.options.hide_icon,
                            "logRequests": sum.options.log_requests,
                            "checkForUpdates": sum.options.check_for_updates
                        }
                    }
                }
            ])
            .to_string()
        }
        "getRules" => {
            let rules_path = RulesFile::default_path(profile_resolver.home_dir());
            let rf = RulesFile::load_from_path(&rules_path).unwrap_or_default();
            let path_str = rules_path.exists().then(|| rules_path.display().to_string());
            serde_json::json!({
                "type": "rules",
                "message": {
                    "defaultBrowser": rf.default_browser,
                    "defaultProfile": rf.default_profile,
                    "options": rf.options,
                    "rules": rf.rules,
                    "path": path_str
                }
            })
            .to_string()
        }
        "saveRules" => {
            let Some(payload) = msg.get("payload") else {
                return serde_json::json!({
                    "type": "saveRulesError",
                    "message": { "error": "Missing payload" }
                })
                .to_string();
            };
            match serde_json::from_value::<RulesFile>(payload.clone()) {
                Ok(rf) => {
                    let rules_path = RulesFile::default_path(profile_resolver.home_dir());
                    if let Err(e) = rf.save_to_path(&rules_path) {
                        return serde_json::json!({
                            "type": "saveRulesError",
                            "message": { "error": e }
                        })
                        .to_string();
                    }
                    let mut eng = engine.lock().unwrap();
                    eng.update_rules(&rf);
                    serde_json::json!({
                        "type": "rules",
                        "message": {
                            "defaultBrowser": rf.default_browser,
                            "defaultProfile": rf.default_profile,
                            "options": rf.options,
                            "rules": rf.rules,
                            "path": rules_path.display().to_string()
                        }
                    })
                    .to_string()
                }
                Err(e) => serde_json::json!({
                    "type": "saveRulesError",
                    "message": { "error": e.to_string() }
                })
                .to_string(),
            }
        }
        "testUrl" => {
            let url = msg.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let eng = engine.lock().unwrap();
            match eng.resolve(url, None, false) {
                Ok(cfg) => {
                    let plan = profile_resolver.build_launch_plan(
                        &cfg,
                        false,
                        eng.summary().options.on_missing_profile,
                    );
                    serde_json::json!({
                        "type": "testUrlResult",
                        "message": {
                            "url": plan.final_url,
                            "browser": plan.browser_name,
                            "openInBackground": plan.open_in_background,
                            "profile": cfg.profile,
                            "args": plan.args,
                            "warning": plan.warning
                        }
                    })
                    .to_string()
                }
                Err(e) => serde_json::json!({
                    "type": "testUrlResult",
                    "message": { "error": e }
                })
                .to_string(),
            }
        }
        "getInstalledBrowsers" => {
            let names: Vec<&str> = BROWSERS_REGISTRY.iter().map(|b| b.app_name).collect();
            serde_json::json!({
                "type": "installedBrowsers",
                "message": names
            })
            .to_string()
        }
        "getBrowserProfiles" => {
            let browser = msg.get("browser").and_then(|v| v.as_str()).unwrap_or("");
            let profiles = profile_resolver.list_profiles(browser);
            serde_json::json!({
                "type": "browserProfiles",
                "message": {
                    "browser": browser,
                    "profiles": profiles
                }
            })
            .to_string()
        }
        _ => "[]".to_string(),
    }
}

fn guess_mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn write_http_response(
    stream: &mut impl Write,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

const FALLBACK_SPA_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Browsify — URL Tester & Rules Manager</title>
<style>
  body { font-family: system-ui, -apple-system, sans-serif; background: #0d1117; color: #e6edf3; max-width: 780px; margin: 32px auto; padding: 0 16px; }
  .card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 16px; margin-bottom: 16px; }
  input, button { padding: 8px 12px; border-radius: 6px; border: 1px solid #30363d; background: #0d1117; color: #e6edf3; font-size: 14px; }
  button { background: #238636; border-color: #2ea043; cursor: pointer; font-weight: 600; }
  pre { background: #0d1117; padding: 12px; border-radius: 6px; overflow-x: auto; font-size: 13px; }
</style>
</head>
<body>
  <h2>Browsify — On-Demand Localhost UI</h2>
  <div class="card">
    <h3>Test URL Routing & Profile Resolution</h3>
    <div style="display:flex;gap:8px;margin-top:8px;">
      <input id="urlInput" style="flex:1" value="https://github.com/myorg/repo?utm_source=slack" />
      <button onclick="runTest()">Test URL</button>
    </div>
    <pre id="resultBox">Click "Test URL" to inspect resolved browser, profile, and launch flags.</pre>
  </div>
  <script>
    async function runTest() {
      const url = document.getElementById("urlInput").value;
      const res = await fetch("/api/rpc", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ type: "testUrl", url })
      });
      const data = await res.json();
      document.getElementById("resultBox").textContent = JSON.stringify(data.message, null, 2);
    }
  </script>
</body>
</html>
"#;
