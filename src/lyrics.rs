//! Lyrics: where they come from, and how they are read.

pub mod lrc;
pub mod lrclib;
pub mod normalize;

use std::time::Duration;

/// One line of a song, anchored at the moment it should appear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub at: Duration,
    pub kind: LineKind,
}

/// A blank line in an LRC file is not a line with no words: it is the moment
/// the screen has to go quiet. Spelling that out keeps the overlay from
/// leaving the last line frozen through a whole instrumental.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    Sung(String),
    Instrumental,
}

impl Line {
    /// The words to show, or `None` while nothing is being sung.
    pub fn sung(&self) -> Option<&str> {
        match &self.kind {
            LineKind::Sung(text) => Some(text),
            LineKind::Instrumental => None,
        }
    }
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
