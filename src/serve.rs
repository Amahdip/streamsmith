//! The built-in preview server.
//!
//! A tiny synchronous static-file server over the output directory, with `/`
//! serving an embedded hls.js player pointed at the master playlist. This is the
//! payoff: one command and the adaptive stream is playing in your browser.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tiny_http::{Header, Response, Server};

use crate::package::MASTER_PLAYLIST;
use crate::ui;

/// The player page, baked into the binary at compile time.
const PLAYER_HTML: &str = include_str!("../assets/player.html");

/// hls.js (Apache-2.0), vendored and served locally — the preview works fully
/// offline and loads nothing from a CDN.
const HLS_JS: &str = include_str!("../assets/hls.min.js");

/// Serve `dir` on `127.0.0.1:port` until interrupted. Optionally opens a browser.
pub fn run(dir: &str, port: u16, open_browser: bool) -> Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).map_err(|e| anyhow::anyhow!("could not bind {addr}: {e}"))?;

    let url = format!("http://{addr}/");
    ui::serving(&url);
    if open_browser {
        // Best-effort; a headless environment simply prints the URL above.
        let _ = open::that(&url);
    }

    let root =
        fs::canonicalize(dir).with_context(|| format!("resolving output directory {dir}"))?;

    for request in server.incoming_requests() {
        let response = build_response(&root, request.url());
        // A broken pipe (browser closed the tab) is not a server error.
        let _ = request.respond(response);
    }
    Ok(())
}

/// Route a request to the player page or a file under `root`.
fn build_response(root: &Path, raw_url: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    // Strip the query string and decode the path.
    let path_part = raw_url.split('?').next().unwrap_or("/");

    if path_part == "/" || path_part == "/index.html" {
        // Inject the master-playlist filename so the player knows what to load.
        let html = PLAYER_HTML.replace("{{MASTER}}", MASTER_PLAYLIST);
        return html_response(html);
    }

    // The vendored player library, served from the binary itself.
    if path_part == "/hls.js" {
        return Response::from_data(HLS_JS.as_bytes().to_vec()).with_header(header(
            "Content-Type",
            "application/javascript; charset=utf-8",
        ));
    }

    match resolve(root, path_part) {
        Some(path) => match fs::read(&path) {
            Ok(bytes) => file_response(&path, bytes),
            Err(_) => not_found(),
        },
        None => not_found(),
    }
}

/// Safely resolve a request path to a file inside `root`, rejecting any
/// `..` traversal that would escape the served directory.
fn resolve(root: &Path, url_path: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for segment in url_path.trim_start_matches('/').split('/') {
        // Reject empty, dot, dot-dot, or any embedded separator/NUL.
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains(['/', '\\', '\0'])
        {
            return None;
        }
        path.push(segment);
    }
    // Defense in depth: even if a tricky segment slipped through, the resolved
    // real path must still live under the served root.
    let canonical = fs::canonicalize(&path).ok()?;
    canonical.starts_with(root).then_some(canonical)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("m3u8") => "application/vnd.apple.mpegurl",
        Some("ts") => "video/mp2t",
        Some("mpd") => "application/dash+xml",
        Some("m4s") => "video/iso.segment",
        Some("mp4") => "video/mp4",
        Some("html") => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn header(name: &str, value: &str) -> Header {
    // These are compile-time-valid header strings.
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid header")
}

fn html_response(html: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(html.into_bytes())
        .with_header(header("Content-Type", "text/html; charset=utf-8"))
}

// Note: no CORS headers on purpose. The player is same-origin, and a wildcard
// Access-Control-Allow-Origin would let any website the user visits while
// previewing read the stream off 127.0.0.1.
fn file_response(path: &Path, bytes: Vec<u8>) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(bytes).with_header(header("Content-Type", content_type(path)))
}

fn not_found() -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string("404 Not Found").with_status_code(404)
}
