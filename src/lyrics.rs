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

    /// The line being sung at `position`, or `None` before the first one.
    ///
    /// An instrumental gap is a line like any other here: it is found, and it
    /// has no words, which is how the screen goes quiet.
    pub fn line_at(&self, position: Duration) -> Option<&Line> {
        let position = self.shifted(position);
        // The last line whose moment has already passed.
        let index = self.lines.partition_point(|line| line.at <= position);
        index.checked_sub(1).map(|index| &self.lines[index])
    }

    /// The next lines to be sung, skipping the silences.
    ///
    /// An instrumental gap is left out on purpose: showing a blank line in a
    /// list of what is coming says nothing.
    pub fn after(&self, position: Duration, how_many: usize) -> Vec<String> {
        let position = self.shifted(position);
        let start = self.lines.partition_point(|line| line.at <= position);
        self.lines[start..]
            .iter()
            .filter_map(|line| line.sung())
            .take(how_many)
            .map(str::to_owned)
            .collect()
    }

    /// The file's own `[offset:]`, applied to a position before looking it up.
    fn shifted(&self, position: Duration) -> Duration {
        let offset = Duration::from_millis(self.offset_ms.unsigned_abs());
        if self.offset_ms >= 0 {
            position + offset
        } else {
            position.saturating_sub(offset)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song() -> Lyrics {
        lrc::parse("[00:10.00]first\n[00:20.00]\n[00:30.00]second")
    }

    #[test]
    fn nothing_is_sung_before_the_first_line() {
        assert!(song().line_at(Duration::from_secs(5)).is_none());
    }

    #[test]
    fn a_line_holds_until_the_next_one() {
        let song = song();
        assert_eq!(
            song.line_at(Duration::from_secs(10)).unwrap().sung(),
            Some("first")
        );
        assert_eq!(
            song.line_at(Duration::from_secs(19)).unwrap().sung(),
            Some("first")
        );
    }

    #[test]
    fn a_gap_clears_the_screen() {
        assert_eq!(
            song().line_at(Duration::from_secs(25)).unwrap().sung(),
            None
        );
    }

    #[test]
    fn the_last_line_stays_to_the_end() {
        assert_eq!(
            song().line_at(Duration::from_secs(600)).unwrap().sung(),
            Some("second")
        );
    }

    #[test]
    fn the_files_own_offset_moves_the_lookup() {
        let mut song = song();
        // The file says its lyrics run half a second late.
        song.offset_ms = 500;
        assert_eq!(
            song.line_at(Duration::from_millis(9_600)).unwrap().sung(),
            Some("first")
        );
    }

    #[test]
    fn the_lines_still_to_come_skip_the_silences() {
        let song = song();
        assert_eq!(song.after(Duration::from_secs(0), 2), ["first", "second"]);
        assert_eq!(song.after(Duration::from_secs(12), 2), ["second"]);
        assert!(song.after(Duration::from_secs(600), 2).is_empty());
    }

    #[test]
    fn an_empty_song_has_no_lines() {
        assert!(Lyrics::default().line_at(Duration::from_secs(1)).is_none());
    }
}
