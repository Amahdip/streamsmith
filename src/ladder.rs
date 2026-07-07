//! Planning the adaptive-bitrate (ABR) ladder.
//!
//! Given what the source *is*, decide which renditions to produce. Rules:
//! * emit every standard tier at or below the source height — never upscale;
//! * always keep at least the lowest tier so even a tiny source is playable;
//! * derive each rendition's width from the *source* aspect ratio (rounded to
//!   an even number, as H.264 requires), so non-16:9 inputs stay correct.

use crate::media::MediaInfo;

/// One rung of the ladder: a single output quality.
#[derive(Debug, Clone)]
pub struct Rendition {
    /// Label used for filenames and the playlist, e.g. `"720p"`.
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Target average video bitrate.
    pub v_kbps: u32,
    /// Peak (cap) video bitrate — bounds spikes for smooth ABR switching.
    pub maxrate_kbps: u32,
    /// VBV buffer size.
    pub bufsize_kbps: u32,
    /// Audio bitrate.
    pub a_kbps: u32,
}

impl Rendition {
    /// Peak bandwidth advertised to players in the master playlist (bits/sec).
    ///
    /// RFC 8216 §4.3.4.2 wants the *peak segment* bitrate, which is more than
    /// the elementary-stream rates: MPEG-TS packetization adds roughly 10–15%
    /// overhead at these bitrates. Advertise video cap + audio + 15% so players
    /// don't switch up to a rung they can't actually sustain.
    pub fn bandwidth_bps(&self) -> u32 {
        (self.maxrate_kbps + self.a_kbps) * 1000 * 115 / 100
    }
}

/// Build the ladder for a source.
pub fn plan(info: &MediaInfo) -> Vec<Rendition> {
    /// `(label, height, target_video_kbps)` — ascending.
    const TIERS: &[(&str, u32, u32)] = &[
        ("240p", 240, 400),
        ("360p", 360, 800),
        ("480p", 480, 1_400),
        ("720p", 720, 2_800),
        ("1080p", 1_080, 5_000),
        ("1440p", 1_440, 9_000),
        ("2160p", 2_160, 18_000),
    ];

    let source_height = info.height.max(240);

    TIERS
        .iter()
        .filter(|(_, height, _)| *height <= source_height)
        .map(|(name, height, v_kbps)| Rendition {
            name: (*name).to_string(),
            width: width_for(info, *height),
            height: *height,
            v_kbps: *v_kbps,
            // ~7% headroom over the average for the peak cap.
            maxrate_kbps: v_kbps * 107 / 100,
            // 1.5× the target is a common, safe VBV buffer.
            bufsize_kbps: v_kbps * 3 / 2,
            a_kbps: 128,
        })
        .collect()
}

/// Width that preserves the source aspect ratio at `height`, computed exactly
/// the way ffmpeg's `scale=-2:H` does (round to nearest integer, then align
/// *up* to even — FFALIGN), so the master playlist's `RESOLUTION` tag equals
/// the actual encoded size. Falls back to 16:9 when the source dimensions are
/// unknown.
fn width_for(info: &MediaInfo, height: u32) -> u32 {
    let raw = if info.height > 0 {
        height as f64 * info.width as f64 / info.height as f64
    } else {
        height as f64 * 16.0 / 9.0
    };
    let rounded = raw.round() as u32;
    // FFALIGN(x, 2): round up to the next even number.
    ((rounded + 1) & !1).max(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(w: u32, h: u32) -> MediaInfo {
        MediaInfo {
            width: w,
            height: h,
            fps: 30.0,
            duration: 10.0,
            v_codec: "h264".into(),
            has_audio: true,
        }
    }

    #[test]
    fn never_upscales() {
        let ladder = plan(&source(1280, 720));
        assert!(ladder.iter().all(|r| r.height <= 720));
        assert_eq!(ladder.last().unwrap().name, "720p");
    }

    #[test]
    fn tiny_source_still_yields_one_rung() {
        let ladder = plan(&source(160, 120));
        assert_eq!(ladder.len(), 1);
        assert_eq!(ladder[0].name, "240p");
    }

    #[test]
    fn width_matches_ffmpeg_ffalign_rounding() {
        // 1279×720 source at the 480p rung: 852.67 → round 853 → align UP 854,
        // which is what ffmpeg's `scale=-2:480` actually encodes.
        let ladder = plan(&source(1279, 720));
        let r480 = ladder.iter().find(|r| r.name == "480p").unwrap();
        assert_eq!(r480.width, 854);
    }

    #[test]
    fn bandwidth_includes_container_overhead() {
        let ladder = plan(&source(1920, 1080));
        let r720 = ladder.iter().find(|r| r.name == "720p").unwrap();
        // (2996 maxrate + 128 audio) kbps + 15% TS overhead.
        assert_eq!(r720.bandwidth_bps(), (2996 + 128) * 1000 * 115 / 100);
    }

    #[test]
    fn widths_follow_source_aspect_and_are_even() {
        // A 4:3 source: 480p rung should be 640x480, not 854x480.
        let ladder = plan(&source(1440, 1080));
        let r480 = ladder.iter().find(|r| r.name == "480p").unwrap();
        assert_eq!((r480.width, r480.height), (640, 480));
        assert!(ladder.iter().all(|r| r.width % 2 == 0));
    }
}
