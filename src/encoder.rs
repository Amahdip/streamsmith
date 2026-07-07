//! Choosing the video encoder — software (libx264) or hardware.
//!
//! streamsmith defaults to `libx264` because it's universal and predictable, but
//! every rung of the ladder is an independent encode, so a hardware encoder
//! (Apple VideoToolbox, NVIDIA NVENC, Intel QuickSync) can cut wall-clock time
//! dramatically. `--hwaccel auto` picks the best encoder actually present in the
//! local ffmpeg build; an explicit choice errors loudly if it isn't available.

use std::collections::HashSet;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::ladder::Rendition;

/// What the user asked for on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwAccel {
    /// Use the best hardware encoder present, else fall back to libx264.
    Auto,
    /// Force software libx264.
    Off,
    VideoToolbox,
    Nvenc,
    Qsv,
}

impl HwAccel {
    /// Parse the `--hwaccel` value (with a few friendly aliases).
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "auto" => Self::Auto,
            "off" | "none" | "software" | "cpu" | "x264" => Self::Off,
            "videotoolbox" | "vt" | "apple" => Self::VideoToolbox,
            "nvenc" | "nvidia" | "cuda" => Self::Nvenc,
            "qsv" | "intel" | "quicksync" => Self::Qsv,
            other => {
                bail!("unknown --hwaccel '{other}' (expected: auto, off, videotoolbox, nvenc, qsv)")
            }
        })
    }
}

/// Internal per-family behavior — each encoder expresses rate control and
/// keyframe control a little differently.
#[derive(Debug, Clone, Copy)]
enum Family {
    X264,
    VideoToolbox,
    Nvenc,
    Qsv,
}

/// A resolved encoder, ready to emit ffmpeg arguments.
#[derive(Debug, Clone)]
pub struct Encoder {
    /// ffmpeg `-c:v` value, e.g. `"libx264"` or `"h264_videotoolbox"`.
    pub codec: &'static str,
    /// Human label for the progress line, e.g. `"VideoToolbox (hardware)"`.
    pub label: &'static str,
    family: Family,
}

impl Encoder {
    fn x264() -> Self {
        Self {
            codec: "libx264",
            label: "libx264 (software)",
            family: Family::X264,
        }
    }
    fn videotoolbox() -> Self {
        Self {
            codec: "h264_videotoolbox",
            label: "VideoToolbox (hardware)",
            family: Family::VideoToolbox,
        }
    }
    fn nvenc() -> Self {
        Self {
            codec: "h264_nvenc",
            label: "NVENC (hardware)",
            family: Family::Nvenc,
        }
    }
    fn qsv() -> Self {
        Self {
            codec: "h264_qsv",
            label: "QuickSync (hardware)",
            family: Family::Qsv,
        }
    }

    /// The full `-c:v … <rate control> <keyframes>` argument list for one
    /// rendition. `preset` is only consulted by libx264; hardware encoders
    /// ignore it.
    pub fn video_args(&self, r: &Rendition, gop: u32, preset: &str) -> Vec<String> {
        let mut args = vec![
            "-c:v".into(),
            self.codec.into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
        ];

        // Rate control is the same request for every family: an average target,
        // a peak cap, and a VBV buffer.
        let rate = |args: &mut Vec<String>| {
            args.extend([
                "-b:v".into(),
                format!("{}k", r.v_kbps),
                "-maxrate".into(),
                format!("{}k", r.maxrate_kbps),
                "-bufsize".into(),
                format!("{}k", r.bufsize_kbps),
            ]);
        };

        match self.family {
            Family::X264 => {
                args.extend(["-preset".into(), preset.to_string()]);
                rate(&mut args);
                // Fixed GOP, no scene-cut keyframes → segment-aligned keyframes.
                args.extend([
                    "-g".into(),
                    gop.to_string(),
                    "-keyint_min".into(),
                    gop.to_string(),
                    "-sc_threshold".into(),
                    "0".into(),
                ]);
            }
            // Hardware families: keep flags to the widely-supported common set
            // (a forced GOP for clean segment boundaries). Encoder-specific
            // tuning knobs vary by ffmpeg version, so we deliberately stay
            // conservative for portability.
            Family::VideoToolbox | Family::Nvenc | Family::Qsv => {
                rate(&mut args);
                args.extend(["-g".into(), gop.to_string()]);
            }
        }
        args
    }
}

/// Resolve the requested acceleration against what this ffmpeg build offers.
pub fn resolve(ffmpeg_bin: &str, requested: HwAccel) -> Result<Encoder> {
    let available = list_encoders(ffmpeg_bin)?;
    let has = |name: &str| available.contains(name);

    // For an explicit request, fail clearly if the encoder isn't compiled in —
    // better than silently producing software output the user didn't expect.
    let require = |name: &'static str, enc: Encoder| -> Result<Encoder> {
        if has(name) {
            Ok(enc)
        } else {
            bail!(
                "requested encoder '{name}' is not available in this ffmpeg build \
                 (try `--hwaccel auto`, or a build with it enabled)"
            )
        }
    };

    match requested {
        HwAccel::Off => Ok(Encoder::x264()),
        HwAccel::VideoToolbox => require("h264_videotoolbox", Encoder::videotoolbox()),
        HwAccel::Nvenc => require("h264_nvenc", Encoder::nvenc()),
        HwAccel::Qsv => require("h264_qsv", Encoder::qsv()),
        HwAccel::Auto => Ok(if has("h264_videotoolbox") {
            Encoder::videotoolbox()
        } else if has("h264_nvenc") {
            Encoder::nvenc()
        } else if has("h264_qsv") {
            Encoder::qsv()
        } else {
            Encoder::x264()
        }),
    }
}

/// The set of encoder names this ffmpeg build advertises (`ffmpeg -encoders`).
fn list_encoders(ffmpeg_bin: &str) -> Result<HashSet<String>> {
    let output = Command::new(ffmpeg_bin)
        .args(["-hide_banner", "-encoders"])
        .output()
        .with_context(|| {
            format!("could not run `{ffmpeg_bin}` — is FFmpeg installed and on your PATH?")
        })?;

    // The listing looks like: ` V....D h264_videotoolbox   VideoToolbox H.264 ...`
    // The encoder name is the second whitespace-separated token per line.
    let text = String::from_utf8_lossy(&output.stdout);
    let names = text
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            // Data lines start with capability flags (e.g. "V....D"); skip the
            // header and separators that don't.
            let mut it = line.split_whitespace();
            let flags = it.next()?;
            if flags.len() >= 2 && flags.starts_with('V') {
                it.next().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ladder::Rendition;

    fn rung() -> Rendition {
        Rendition {
            name: "720p".into(),
            width: 1280,
            height: 720,
            v_kbps: 2800,
            maxrate_kbps: 2996,
            bufsize_kbps: 4200,
            a_kbps: 128,
        }
    }

    #[test]
    fn parses_hwaccel_aliases() {
        assert_eq!(HwAccel::parse("auto").unwrap(), HwAccel::Auto);
        assert_eq!(HwAccel::parse("VT").unwrap(), HwAccel::VideoToolbox);
        assert_eq!(HwAccel::parse("cpu").unwrap(), HwAccel::Off);
        assert!(HwAccel::parse("magic").is_err());
    }

    #[test]
    fn x264_args_include_preset_and_scene_cut_control() {
        let args = Encoder::x264().video_args(&rung(), 180, "veryfast");
        assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|w| w == ["-preset", "veryfast"]));
        assert!(args.windows(2).any(|w| w == ["-sc_threshold", "0"]));
        assert!(args.windows(2).any(|w| w == ["-b:v", "2800k"]));
    }

    #[test]
    fn hardware_args_skip_x264_only_flags() {
        let args = Encoder::videotoolbox().video_args(&rung(), 180, "veryfast");
        assert!(args.windows(2).any(|w| w == ["-c:v", "h264_videotoolbox"]));
        // No libx264-specific flags should leak into a hardware encode.
        assert!(!args.iter().any(|a| a == "-preset"));
        assert!(!args.iter().any(|a| a == "-sc_threshold"));
        // Rate control and forced GOP still apply.
        assert!(args.windows(2).any(|w| w == ["-maxrate", "2996k"]));
        assert!(args.windows(2).any(|w| w == ["-g", "180"]));
    }
}
