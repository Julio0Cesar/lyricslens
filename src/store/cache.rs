//! Lyrics already fetched, kept on disk so the same song works offline.
//!
//! One file per recording, holding the LRC exactly as the service sent it:
//! readable, editable by hand, and re-parsed on the way back in.

use std::path::PathBuf;
use std::time::Duration;

use crate::lyrics::{Lyrics, lrc};

/// Names the file for a recording.
///
/// The duration is part of the name on purpose: a live version and the album
/// one share artist and title, and they are not the same lyrics.
fn key(artist: &str, title: &str, length: Option<Duration>) -> String {
    let mut hash = Fnv::new();
    hash.write(artist.trim().to_lowercase().as_bytes());
    hash.write(b"\0");
    hash.write(title.trim().to_lowercase().as_bytes());
    if let Some(length) = length {
        hash.write(b"\0");
        hash.write(length.as_secs().to_string().as_bytes());
    }
    format!("{:016x}.lrc", hash.finish())
}

fn path(artist: &str, title: &str, length: Option<Duration>) -> Option<PathBuf> {
    Some(super::cache_dir()?.join(key(artist, title, length)))
}

/// The lyrics saved for this recording, if any were.
pub fn get(artist: &str, title: &str, length: Option<Duration>) -> Option<Lyrics> {
    let path = path(artist, title, length)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let lyrics = lrc::parse(&text);
    if lyrics.is_empty() {
        return None;
    }
    tracing::debug!(path = %path.display(), "lyrics read from the cache");
    Some(lyrics)
}

/// Keeps the LRC for next time. A cache that cannot be written is not an error.
pub fn put(artist: &str, title: &str, length: Option<Duration>, lrc: &str) {
    let Some(path) = path(artist, title, length) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(parent).and_then(|()| std::fs::write(&path, lrc)) {
        tracing::debug!(%error, path = %path.display(), "could not cache the lyrics");
    }
}

/// FNV-1a, 64 bits.
///
/// The standard hasher is explicitly allowed to change between releases, and a
/// cache whose file names move on a compiler upgrade is a cache that is thrown
/// away for no reason.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_recording_gets_the_same_name() {
        let a = key("Radiohead", "Creep", Some(Duration::from_secs(238)));
        let b = key("  radiohead ", "creep", Some(Duration::from_secs(238)));
        assert_eq!(a, b);
    }

    #[test]
    fn a_different_length_is_a_different_recording() {
        let album = key("Radiohead", "Creep", Some(Duration::from_secs(238)));
        let live = key("Radiohead", "Creep", Some(Duration::from_secs(300)));
        assert_ne!(album, live);
    }

    #[test]
    fn an_unknown_length_still_gets_a_name() {
        assert!(key("Radiohead", "Creep", None).ends_with(".lrc"));
    }

    #[test]
    fn the_name_is_stable_across_runs() {
        // Written down so a change to the hash is a deliberate one: every
        // cached file becomes unreachable the day this number moves.
        assert_eq!(
            key("Radiohead", "Creep", Some(Duration::from_secs(238))),
            "0b4c939c07976d46.lrc"
        );
    }

    #[test]
    fn what_was_written_comes_back() {
        let dir = std::env::temp_dir().join("lyricslens-cache-test");
        unsafe { std::env::set_var("XDG_CACHE_HOME", &dir) };

        put("Tester", "A Song", None, "[00:01.00]hello");
        let lyrics = get("Tester", "A Song", None).expect("it was just written");

        unsafe { std::env::remove_var("XDG_CACHE_HOME") };
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(lyrics.lines[0].sung(), Some("hello"));
    }

    #[test]
    fn a_recording_never_seen_is_not_in_the_cache() {
        assert!(get("Nobody", "Nothing At All", None).is_none());
    }
}
