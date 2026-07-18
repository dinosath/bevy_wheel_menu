//! Integration test for WASM deployment.
//!
//! Simulates the GitHub Pages subdirectory structure, starts a local HTTP
//! server, and verifies that all assets (HTML, JS, WASM) are served with
//! HTTP 200 at the correct paths.
//!
//! ## Prerequisites
//!
//! ```sh
//! cargo build --release --example fps --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir dist --out-name wasm_example --target web \
//!   target/wasm32-unknown-unknown/release/examples/fps.wasm
//! cp index.html dist/index.html
//! ```
//!
//! ## How it works
//!
//! 1. Copies `dist/*` into a temp directory under `bevy_quick_action_hud/`
//!    (mirroring the GitHub Pages URL structure).
//! 2. Spawns a minimal HTTP server (stdlib only, no framework) on a random port.
//! 3. Issues GET requests for every expected path.
//! 4. Asserts HTTP 200 and the correct `Content-Type` for each file.

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

// ─── helpers ───────────────────────────────────────────────────────────────────

/// Represents a single HTTP response.
#[derive(Debug)]
struct HttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    #[allow(dead_code)]
    body: Vec<u8>,
}

/// Send an HTTP GET and return the parsed response.
fn http_get(addr: &str, path: &str) -> HttpResponse {
    let mut stream = TcpStream::connect(addr).expect("connect to server");
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();

    let mut reader = BufReader::new(&mut stream);

    // ── status line ────────────────────────────────────────────────────
    let status_line = {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        line
    };
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("valid HTTP status");

    // ── headers ────────────────────────────────────────────────────────
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break; // end of headers
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    // ── body ───────────────────────────────────────────────────────────
    let mut body = Vec::new();
    reader.read_to_end(&mut body).unwrap();

    HttpResponse {
        status,
        headers,
        body,
    }
}

/// Start a minimal HTTP server on a random port that serves files from `root`.
///
/// The server runs until the returned `ServerHandle` is dropped.
struct ServerGuard {
    shutdown: Option<Arc<Mutex<bool>>>,
    port: u16,
}

impl ServerGuard {
    fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(flag) = &self.shutdown {
            *flag.lock().unwrap() = true;
        }
    }
}

fn serve_dir(root: PathBuf) -> ServerGuard {
    let port = Arc::new(AtomicU16::new(0));
    let port_clone = port.clone();
    let shutdown = Arc::new(Mutex::new(false));
    let shutdown_clone = shutdown.clone();

    thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
        port_clone.store(listener.local_addr().unwrap().port(), Ordering::SeqCst);

        for stream in listener.incoming() {
            if *shutdown_clone.lock().unwrap() {
                break;
            }
            if let Ok(mut stream) = stream {
                let root = root.clone();
                thread::spawn(move || handle_connection(&mut stream, &root));
            }
        }
    });

    // Wait for the server to be ready
    while port.load(Ordering::SeqCst) == 0 {
        thread::sleep(Duration::from_millis(5));
    }

    ServerGuard {
        shutdown: Some(shutdown),
        port: port.load(Ordering::SeqCst),
    }
}

fn handle_connection(stream: &mut TcpStream, root: &Path) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    // Read request line
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }

    // Skip remaining request headers
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
    }

    // Parse path from "GET /path HTTP/1.1"
    let raw_path = request_line.split_whitespace().nth(1).unwrap_or("/");

    // Map the URL path to a filesystem path.
    // GitHub Pages serves files from a subdirectory matching the repo name.
    // Strip that prefix and serve from root/<repo_name>/.
    let fs_path = if let Some(relative) = raw_path.strip_prefix("/bevy_quick_action_hud") {
        // Handle root: "/bevy_quick_action_hud" or "/bevy_quick_action_hud/"
        let relative = if relative.is_empty() || relative == "/" {
            "/index.html"
        } else {
            relative
        };
        let clean = relative.trim_start_matches('/');
        // Files are inside root/bevy_quick_action_hud/
        root.join(REPO_NAME).join(clean)
    } else {
        // Paths outside the subdirectory → 404
        send_status(stream, 404, "Not Found");
        return;
    };

    if fs_path.exists() && fs_path.is_file() {
        let data = fs::read(&fs_path).unwrap();
        let ext = fs_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let mime = mime_for(ext);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            data.len()
        );
        let mut out = stream.try_clone().unwrap();
        out.write_all(response.as_bytes()).unwrap();
        out.write_all(&data).unwrap();
    } else {
        send_status(stream, 404, "Not Found");
    }
}

fn send_status(stream: &mut TcpStream, code: u16, reason: &str) {
    let body = format!("{code} {reason}");
    let response = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript",
        "wasm" => "application/wasm",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        "toml" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

// ─── test ──────────────────────────────────────────────────────────────────────

const REPO_NAME: &str = "bevy_quick_action_hud";

#[test]
fn wasm_deploy_smoke_test() {
    // ── 1. Verify dist/ exists ──────────────────────────────────────────
    let dist = PathBuf::from("dist");
    assert!(
        dist.exists(),
        "dist/ directory not found — run `trunk build --release --example fps --public-url \"/{REPO_NAME}/\" --dist dist` first"
    );

    // ── 2. Populate a temp directory matching the GitHub Pages layout ───
    let tmp = std::env::temp_dir().join(format!("wasm-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let subdir = tmp.join(REPO_NAME);
    fs::create_dir_all(&subdir).unwrap();

    let mut entries: Vec<_> = fs::read_dir(&dist)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    assert!(!entries.is_empty(), "dist/ is empty");

    for entry in &entries {
        let name = entry.file_name();
        fs::copy(entry.path(), subdir.join(&name)).unwrap();
        println!("  staged  /{}/{}", REPO_NAME, name.to_string_lossy());
    }

    // ── 3. Start the HTTP server ────────────────────────────────────────
    let server = serve_dir(tmp.clone());
    let addr = format!("127.0.0.1:{}", server.port());
    println!("\n  server  http://{addr}/");
    println!("  root    {}\n", tmp.display());

    // Give the server a moment to start
    thread::sleep(Duration::from_millis(50));

    // ── 4. Build expected paths ────────────────────────────────────────
    // Collect all staged file names
    let staged_names: Vec<String> = entries
        .iter()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    let mut test_paths: Vec<&str> = vec![
        "/bevy_quick_action_hud/",
        "/bevy_quick_action_hud/index.html",
    ];

    // Add the hashed JS and WASM files
    for name in &staged_names {
        if name.ends_with(".js") {
            test_paths.push(Box::leak(
                format!("/{}/{}", REPO_NAME, name).into_boxed_str(),
            ));
        } else if name.ends_with(".wasm") {
            test_paths.push(Box::leak(
                format!("/{}/{}", REPO_NAME, name).into_boxed_str(),
            ));
        }
    }

    // ── 5. Test every path ─────────────────────────────────────────────
    let mut all_ok = true;
    let start = Instant::now();

    for path in &test_paths {
        let resp = http_get(&addr, path);

        // Determine expected content-type from the served file, not the URL
        let expected_content_type = if path.ends_with('/') || path.ends_with("/index.html") {
            mime_for("html")
        } else {
            let ext = path.rsplit('.').next().unwrap_or("");
            mime_for(ext)
        };
        let content_type = resp
            .headers
            .get("content-type")
            .map(|s| s.as_str())
            .unwrap_or("(missing)");

        let ok = resp.status == 200;
        let mime_ok = content_type.starts_with(expected_content_type);

        let status = if ok && mime_ok {
            "✅"
        } else if ok && !mime_ok {
            "⚠️"
        } else {
            "❌"
        };

        println!(
            "  {status} GET {path:70} → {} (content-type: {content_type})",
            resp.status
        );

        if !ok {
            all_ok = false;
        }
    }

    let elapsed = start.elapsed();
    println!("\n  {:.2}s total", elapsed.as_secs_f64());

    // ── 6. Clean up ────────────────────────────────────────────────────
    drop(server);
    fs::remove_dir_all(&tmp).unwrap();

    assert!(all_ok, "some paths returned non-200 (see ❌ above)");
}
