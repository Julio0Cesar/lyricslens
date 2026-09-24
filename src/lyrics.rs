//! Lyrics: where they come from, and how they are read.

pub mod lrc;

use std::time::Duration;

/// One line of a song, anchored at the moment it should appear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub at: Duration,
    pub text: String,
}

/// A whole song, in the order it is sung.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lyrics {
    pub lines: Vec<Line>,
    /// What the file's `[offset:...]` tag asks for, in milliseconds. Positive
    /// means the lyrics run early and have to be pushed later.
    pub offset_ms: i64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

impl Lyrics {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}
