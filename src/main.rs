//! streamsmith — turn any video into a ready-to-serve HLS adaptive stream,
//! with a built-in preview player, in one command.
//!
//! Pipeline: probe the source (ffprobe) → plan an ABR ladder → encode each
//! rendition (ffmpeg) into HLS → write the master playlist → serve a preview.

mod ladder;
mod media;
mod package;
mod probe;
mod serve;
mod ui;

use std::process::ExitCode;
use std::thread::available_parallelism;

use anyhow::{bail, Result};
use clap::Parser;

use package::{PackageOptions, MASTER_PLAYLIST};

/// One command → a web-ready adaptive stream.
#[derive(Debug, Parser)]
#[command(name = "streamsmith", version, about, long_about = None)]
struct Cli {
    /// Input video (a file path, or any URL your ffmpeg build can read).
    input: String,

    /// Directory to write the stream bundle into.
    #[arg(short, long, default_value = "stream")]
    out: String,

    /// HLS segment length, in seconds.
    #[arg(long, default_value_t = 6)]
    segment: u32,

    /// x264 preset (ultrafast … placebo): faster encodes trade off file size.
    #[arg(long, default_value = "veryfast")]
    preset: String,

    /// Max parallel encodes. Defaults to the CPU count.
    #[arg(short, long)]
    jobs: Option<usize>,

    /// Package only — don't start the preview server.
    #[arg(long)]
    no_serve: bool,

    /// Preview server port.
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Don't open a browser automatically.
    #[arg(long)]
    no_open: bool,

    /// Path to the ffmpeg binary.
    #[arg(long, default_value = "ffmpeg")]
    ffmpeg: String,

    /// Path to the ffprobe binary.
    #[arg(long, default_value = "ffprobe")]
    ffprobe: String,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // `{:#}` prints the whole anyhow context chain on one tidy line.
            eprintln!("\x1b[31merror:\x1b[0m {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    ui::banner();

    // 1. Probe the source.
    let info = probe::probe(&cli.ffprobe, &cli.input)?;
    if !info.has_video() {
        bail!("{} has no video stream to package", cli.input);
    }
    ui::step_probe(&info);

    // 2. Plan the ladder.
    let ladder = ladder::plan(&info);
    ui::step_ladder(&ladder);

    // 3. Encode + package into HLS.
    let jobs = cli
        .jobs
        .unwrap_or_else(|| available_parallelism().map(|n| n.get()).unwrap_or(4))
        .max(1);
    // Split the CPU budget across concurrent ffmpeg processes so we don't
    // oversubscribe cores (jobs × threads_per_job ≈ total cores).
    let cores = available_parallelism().map(|n| n.get()).unwrap_or(4);
    let threads_per_job = (cores / jobs.min(ladder.len().max(1)).max(1)).max(1) as u32;

    let opts = PackageOptions {
        ffmpeg_bin: &cli.ffmpeg,
        input: &cli.input,
        out_dir: &cli.out,
        segment_secs: cli.segment,
        preset: &cli.preset,
        threads_per_job,
    };
    package::hls(&opts, &info, &ladder, jobs)?;
    ui::step_done(&cli.out, MASTER_PLAYLIST);

    // 4. Serve a live preview.
    if cli.no_serve {
        return Ok(());
    }
    serve::run(&cli.out, cli.port, !cli.no_open)
}
