//! Knowing where the song is, right now.

pub mod clock;

/// What the player says it is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playback {
    Playing,
    Paused,
    Stopped,
}

impl Playback {
    /// MPRIS spells these three exactly, and anything else is a player bug.
    pub fn from_mpris(status: &str) -> Self {
        match status {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            _ => Self::Stopped,
        }
    }
}
