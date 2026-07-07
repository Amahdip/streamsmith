//! The normalized view of a source asset that the planner needs.
//!
//! `ffprobe` emits a sprawling JSON document; [`crate::probe`] distills it into
//! this flat, strongly-typed struct. Nothing downstream ever sees ffprobe's
//! wire format.

/// Everything streamsmith needs to know about the input to plan a ladder.
#[derive(Debug, Clone)]
pub struct MediaInfo {
    /// Source width in pixels (`0` if there is no video stream).
    pub width: u32,
    /// Source height in pixels (`0` if there is no video stream).
    pub height: u32,
    /// Frames per second (already divided out from ffprobe's `"30000/1001"`).
    pub fps: f64,
    /// Duration in seconds.
    pub duration: f64,
    /// Video codec name, e.g. `"h264"`.
    pub v_codec: String,
    /// Whether the source carries an audio stream.
    pub has_audio: bool,
}

impl MediaInfo {
    /// True when the source actually contains a video stream.
    pub fn has_video(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// A friendly `MM:SS` (or `H:MM:SS`) rendering of the duration.
    pub fn pretty_duration(&self) -> String {
        let total = self.duration.round() as u64;
        let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m}:{s:02}")
        }
    }
}
