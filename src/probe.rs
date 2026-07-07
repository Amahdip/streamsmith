//! Reading source metadata with `ffprobe`.
//!
//! Runs `ffprobe -print_format json -show_format -show_streams` and folds the
//! result into [`MediaInfo`]. The `Raw*` structs mirror ffprobe's JSON exactly
//! (everything optional, numbers as strings) and never escape this module.

use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use crate::media::MediaInfo;

/// Probe `input` and return normalized [`MediaInfo`].
pub fn probe(ffprobe_bin: &str, input: &str) -> Result<MediaInfo> {
    let output = Command::new(ffprobe_bin)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(input)
        .output()
        .with_context(|| {
            format!("could not run `{ffprobe_bin}` — is FFmpeg installed and on your PATH?")
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("ffprobe failed for {input}: {}", stderr.trim()));
    }

    let raw: RawProbe =
        serde_json::from_slice(&output.stdout).context("parsing ffprobe JSON output")?;

    let video = raw
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"));
    let has_audio = raw
        .streams
        .iter()
        .any(|s| s.codec_type.as_deref() == Some("audio"));

    Ok(MediaInfo {
        width: video.and_then(|v| v.width).unwrap_or(0),
        height: video.and_then(|v| v.height).unwrap_or(0),
        fps: video
            .map(|v| parse_ratio(v.r_frame_rate.as_deref()))
            .unwrap_or(0.0),
        duration: raw
            .format
            .duration
            .as_deref()
            .and_then(|d| d.parse().ok())
            .unwrap_or(0.0),
        v_codec: video
            .and_then(|v| v.codec_name.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        has_audio,
    })
}

/// Parse ffprobe's `"num/den"` frame rate into fps. Returns `0.0` on anything
/// malformed or a zero denominator.
fn parse_ratio(raw: Option<&str>) -> f64 {
    let Some(s) = raw else { return 0.0 };
    match s.split_once('/') {
        Some((num, den)) => match (num.parse::<f64>(), den.parse::<f64>()) {
            (Ok(n), Ok(d)) if d != 0.0 => n / d,
            _ => 0.0,
        },
        None => s.parse().unwrap_or(0.0),
    }
}

// --- raw ffprobe JSON shapes ------------------------------------------------

#[derive(Deserialize)]
struct RawProbe {
    #[serde(default)]
    format: RawFormat,
    #[serde(default)]
    streams: Vec<RawStream>,
}

#[derive(Default, Deserialize)]
struct RawFormat {
    duration: Option<String>,
}

#[derive(Deserialize)]
struct RawStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frame_rate_ratios() {
        assert!((parse_ratio(Some("30000/1001")) - 29.97).abs() < 0.01);
        assert_eq!(parse_ratio(Some("25/1")), 25.0);
        assert_eq!(parse_ratio(Some("0/0")), 0.0);
        assert_eq!(parse_ratio(None), 0.0);
    }
}
