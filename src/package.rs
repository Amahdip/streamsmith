//! Driving FFmpeg to produce the stream bundle.
//!
//! The HLS path encodes each rendition with its own `ffmpeg` process (run with
//! bounded parallelism so we saturate the CPU without oversubscribing it), then
//! writes the master playlist ourselves — which keeps full control over the
//! `BANDWIDTH`/`RESOLUTION` tags and gives clean per-rendition progress.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Result};

use crate::ladder::Rendition;
use crate::media::MediaInfo;
use crate::ui;

/// Filename of the HLS master playlist within the output directory.
pub const MASTER_PLAYLIST: &str = "master.m3u8";

/// Inputs shared by every encode in a run.
pub struct PackageOptions<'a> {
    pub ffmpeg_bin: &'a str,
    pub input: &'a str,
    pub out_dir: &'a str,
    pub segment_secs: u32,
    pub preset: &'a str,
    /// Threads handed to each ffmpeg process (total ≈ jobs × this ≈ cores).
    pub threads_per_job: u32,
}

/// Package `ladder` into an HLS bundle under `opts.out_dir` using up to `jobs`
/// concurrent ffmpeg processes. Writes each variant playlist plus the master.
pub fn hls(
    opts: &PackageOptions,
    info: &MediaInfo,
    ladder: &[Rendition],
    jobs: usize,
) -> Result<()> {
    fs::create_dir_all(opts.out_dir)
        .with_context(|| format!("creating output directory {}", opts.out_dir))?;

    // A GOP aligned to the segment length gives every segment a leading
    // keyframe, so players can switch renditions cleanly at segment boundaries.
    let fps = if info.fps > 0.0 { info.fps } else { 30.0 };
    let gop = (fps * opts.segment_secs as f64).round().max(1.0) as u32;

    // Work-stealing pool: `jobs` threads pull the next rendition index until the
    // ladder is exhausted. No barriers, no oversubscription.
    let next = AtomicUsize::new(0);
    let stdout = Mutex::new(()); // serialize progress lines
    let failures = Mutex::new(Vec::<String>::new());
    let worker_count = jobs.clamp(1, ladder.len().max(1));

    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| loop {
                let idx = next.fetch_add(1, Ordering::SeqCst);
                let Some(rendition) = ladder.get(idx) else {
                    break;
                };

                {
                    let _guard = stdout.lock().unwrap();
                    ui::encode_start(rendition);
                }

                let started = Instant::now();
                match encode_rendition(opts, info, rendition, gop) {
                    Ok(()) => {
                        let _guard = stdout.lock().unwrap();
                        ui::encode_done(rendition, started.elapsed());
                    }
                    Err(e) => {
                        failures
                            .lock()
                            .unwrap()
                            .push(format!("{}: {e:#}", rendition.name));
                    }
                }
            });
        }
    });

    let failures = failures.into_inner().unwrap();
    if !failures.is_empty() {
        bail!("encoding failed:\n  - {}", failures.join("\n  - "));
    }

    write_master_playlist(opts.out_dir, ladder).context("writing master playlist")?;
    Ok(())
}

/// Encode a single rendition into `<name>.m3u8` + `<name>_NNN.ts` segments.
fn encode_rendition(
    opts: &PackageOptions,
    info: &MediaInfo,
    r: &Rendition,
    gop: u32,
) -> Result<()> {
    let out = Path::new(opts.out_dir);
    let variant_playlist = out.join(format!("{}.m3u8", r.name));
    let segment_pattern = out.join(format!("{}_%03d.ts", r.name));

    let mut cmd = Command::new(opts.ffmpeg_bin);
    cmd.arg("-y")
        .args(["-i", opts.input])
        .args(["-threads", &opts.threads_per_job.to_string()])
        // --- video ---
        .args([
            "-c:v",
            "libx264",
            "-preset",
            opts.preset,
            "-pix_fmt",
            "yuv420p",
        ])
        .args(["-vf", &format!("scale=-2:{}", r.height)])
        .args(["-b:v", &format!("{}k", r.v_kbps)])
        .args(["-maxrate", &format!("{}k", r.maxrate_kbps)])
        .args(["-bufsize", &format!("{}k", r.bufsize_kbps)])
        // Fixed GOP, no scene-cut keyframes → segment-aligned keyframes.
        .args(["-g", &gop.to_string(), "-keyint_min", &gop.to_string()])
        .args(["-sc_threshold", "0"]);

    // --- audio (or explicitly none) ---
    if info.has_audio {
        cmd.args(["-c:a", "aac", "-b:a", &format!("{}k", r.a_kbps), "-ac", "2"]);
    } else {
        cmd.arg("-an");
    }

    // --- HLS muxer ---
    cmd.args(["-f", "hls"])
        .args(["-hls_time", &opts.segment_secs.to_string()])
        .args(["-hls_playlist_type", "vod"])
        .args(["-hls_flags", "independent_segments"])
        .arg("-hls_segment_filename")
        .arg(&segment_pattern)
        .arg(&variant_playlist);

    // Capture stderr so a failure produces an actionable message instead of a
    // wall of ffmpeg logging.
    let output = cmd
        .output()
        .with_context(|| format!("could not run `{}`", opts.ffmpeg_bin))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(3).collect();
        return Err(anyhow!(
            "ffmpeg exited with {} ({})",
            output.status,
            tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
        ));
    }
    Ok(())
}

/// Write the HLS master playlist referencing every variant.
fn write_master_playlist(out_dir: &str, ladder: &[Rendition]) -> Result<()> {
    let path = Path::new(out_dir).join(MASTER_PLAYLIST);
    let mut f = fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;

    writeln!(f, "#EXTM3U")?;
    writeln!(f, "#EXT-X-VERSION:3")?;
    // Lowest rung first: players start conservatively, then adapt up.
    for r in ladder {
        writeln!(
            f,
            "#EXT-X-STREAM-INF:BANDWIDTH={},RESOLUTION={}x{}",
            r.bandwidth_bps(),
            r.width,
            r.height
        )?;
        writeln!(f, "{}.m3u8", r.name)?;
    }
    Ok(())
}
