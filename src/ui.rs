//! Terminal output.
//!
//! Deliberately dependency-free: a few ANSI helpers, honoring `NO_COLOR`. Keeps
//! the binary tiny while still looking good in a demo GIF.

use std::sync::OnceLock;
use std::time::Duration;

use crate::ladder::Rendition;
use crate::media::MediaInfo;

fn color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        // Respect NO_COLOR (https://no-color.org/) everywhere.
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        // Legacy Windows consoles (cmd.exe / PowerShell 5 conhost) don't
        // enable ANSI processing by default and would print escape garbage.
        // Only color when a modern terminal identifies itself.
        if cfg!(windows) {
            return ["WT_SESSION", "TERM", "TERM_PROGRAM", "ANSICON"]
                .iter()
                .any(|v| std::env::var_os(v).is_some());
        }
        true
    })
}

fn paint(code: &str, text: &str) -> String {
    if color_enabled() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn bold(t: &str) -> String {
    paint("1", t)
}
fn red(t: &str) -> String {
    paint("31", t)
}
fn dim(t: &str) -> String {
    paint("2", t)
}
fn green(t: &str) -> String {
    paint("32", t)
}
fn cyan(t: &str) -> String {
    paint("36", t)
}

/// A fatal error, printed to stderr (color rules apply here too).
pub fn error(msg: impl std::fmt::Display) {
    eprintln!("{} {msg}", red("error:"));
}

/// One-line product banner.
pub fn banner() {
    println!(
        "{} {}",
        bold("streamsmith"),
        dim("· one command → a web-ready adaptive stream")
    );
}

/// Report the probed source.
pub fn step_probe(info: &MediaInfo) {
    println!(
        "{} {}  {}",
        green("✓"),
        bold("probed"),
        dim(&format!(
            "{}×{} · {} · {:.0} fps · {}",
            info.width,
            info.height,
            info.v_codec,
            info.fps,
            info.pretty_duration()
        )),
    );
}

/// Report the planned ladder.
pub fn step_ladder(ladder: &[Rendition]) {
    let rungs: Vec<String> = ladder.iter().map(|r| r.name.clone()).collect();
    println!(
        "{} {}  {}",
        green("✓"),
        bold("ladder"),
        cyan(&rungs.join("  ")),
    );
}

/// Report the resolved video encoder.
pub fn step_encoder(encoder: &crate::encoder::Encoder) {
    println!(
        "{} {}  {}",
        green("✓"),
        bold("encoder"),
        cyan(encoder.label)
    );
}

/// A rendition has started encoding.
pub fn encode_start(r: &Rendition) {
    println!("{} {} {}", dim("→"), dim("encoding"), dim(&r.name));
}

/// A rendition finished encoding.
pub fn encode_done(r: &Rendition, elapsed: Duration) {
    println!(
        "{}   {:<6} {}",
        green("✓"),
        bold(&r.name),
        dim(&format!(
            "{}×{} · {} kb/s · {:.1}s",
            r.width,
            r.height,
            r.v_kbps,
            elapsed.as_secs_f64()
        )),
    );
}

/// Report where the finished bundle was written.
pub fn step_done(out_dir: &str, master: &str) {
    println!(
        "{} {}  {}",
        green("✓"),
        bold("packaged"),
        dim(&format!("{out_dir}/  (play {master})")),
    );
}

/// Announce the preview server.
pub fn serving(url: &str) {
    println!();
    println!("  {} {}", bold("▶ preview"), cyan(url));
    println!("  {}", dim("press Ctrl-C to stop"));
}
