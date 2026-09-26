//! The album art that sits beside the lyrics.
//!
//! Players are supposed to name a picture in `mpris:artUrl`, and the good ones
//! do: a local file for a local library, an http address for a streaming
//! service. Browsers name nothing, which is the case that matters most here,
//! so what the player does not give is looked up by album.
//!
//! The art belongs to the album, so both the lookup and the cache are keyed by
//! artist and album. Every track of a record then costs one request at most.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::Error;
use crate::media::Track;
use crate::store::cache::digest;

/// Where the lookup goes when the player named no picture. It needs no key and
/// no account, and it answers for nearly anything with a release behind it.
const SEARCH: &str = "https://itunes.apple.com/search";

/// The size asked for. The service names it inside the address it returns, so
/// the small one it offers by default is rewritten to this.
const WANTED: &str = "300x300bb";
const OFFERED: &str = "100x100bb";

/// Anything bigger than this is not a cover, and decoding it would cost more
/// than it is worth.
const LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Clone)]
pub struct Art {
    http: reqwest::Client,
    base: String,
}

#[derive(Debug, Deserialize)]
struct Answer {
    #[serde(default)]
    results: Vec<Release>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Release {
    #[serde(default)]
    artwork_url100: Option<String>,
}

impl Art {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            http: reqwest::Client::builder()
                .user_agent(crate::lyrics::lrclib::AGENT)
                .build()?,
            base: SEARCH.to_owned(),
        })
    }

    /// A picture for this track on disk, or nothing when there is none to find.
    ///
    /// Never an error: the overlay without a cover is the overlay as it has
    /// always been, and a failed lookup should not read as a fault.
    pub async fn find(&self, track: &Track) -> Option<PathBuf> {
        // A local library already has the picture next to the music. Nothing
        // is copied: the player named a file that is going to stay there.
        if let Some(local) = track.art_url.as_deref().and_then(local_file) {
            return local.is_file().then_some(local);
        }

        let (artist, album) = key(track)?;
        let path = crate::store::cache_dir()?.join("art").join(format!(
            "{}.img",
            digest(&[artist.as_bytes(), album.as_bytes()])
        ));
        if path.is_file() {
            return Some(path);
        }

        let address = match track.art_url.as_deref() {
            Some(url) if url.starts_with("http") => url.to_owned(),
            _ => self.look_up(&artist, &album).await?,
        };
        self.download(&address, &path).await
    }

    /// Asks the service which picture belongs to this record.
    async fn look_up(&self, artist: &str, album: &str) -> Option<String> {
        let answer: Answer = self
            .http
            .get(&self.base)
            .query(&[
                ("term", format!("{artist} {album}")),
                ("entity", "album".to_owned()),
                ("limit", "1".to_owned()),
            ])
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json()
            .await
            .ok()?;

        let found = answer.results.into_iter().next()?.artwork_url100?;
        Some(found.replace(OFFERED, WANTED))
    }

    /// Fetches the picture and keeps it. The file on disk is the answer.
    async fn download(&self, address: &str, path: &Path) -> Option<PathBuf> {
        let response = self
            .http
            .get(address)
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        if response.content_length().is_some_and(|size| size > LIMIT) {
            tracing::debug!(address, "the cover is too big to be one");
            return None;
        }

        let bytes = response.bytes().await.ok()?;
        if bytes.is_empty() || bytes.len() as u64 > LIMIT {
            return None;
        }

        let parent = path.parent()?;
        if let Err(error) =
            std::fs::create_dir_all(parent).and_then(|()| std::fs::write(path, &bytes))
        {
            tracing::debug!(%error, path = %path.display(), "could not keep the cover");
            return None;
        }
        Some(path.to_owned())
    }
}

/// What the art is filed under: the album when there is one, the track when
/// there is not, so a single is not stored under every other single's name.
fn key(track: &Track) -> Option<(String, String)> {
    let artist = track.artists.first()?.trim().to_lowercase();
    let album = track
        .album
        .as_deref()
        .or(track.title.as_deref())?
        .trim()
        .to_lowercase();
    (!artist.is_empty() && !album.is_empty()).then_some((artist, album))
}

/// The path inside a `file://` address, with its escapes undone.
fn local_file(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    // `file:///home/...` is the usual shape; a host between the slashes is
    // allowed by the spec and is never a file this machine can open.
    let path = rest.strip_prefix('/').map(|path| format!("/{path}"))?;
    Some(PathBuf::from(unescape(&path)))
}

fn unescape(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' if at + 2 < bytes.len() => {
                let pair = std::str::from_utf8(&bytes[at + 1..at + 3]).unwrap_or("");
                match u8::from_str_radix(pair, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        at += 3;
                    }
                    Err(_) => {
                        out.push(bytes[at]);
                        at += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(artist: &str, album: Option<&str>, title: &str) -> Track {
        Track {
            title: Some(title.to_owned()),
            artists: vec![artist.to_owned()],
            album: album.map(ToOwned::to_owned),
            ..Track::default()
        }
    }

    #[test]
    fn a_local_address_becomes_a_path() {
        assert_eq!(
            local_file("file:///home/me/Music/OK%20Computer/cover.jpg"),
            Some(PathBuf::from("/home/me/Music/OK Computer/cover.jpg"))
        );
    }

    #[test]
    fn a_remote_address_is_not_a_path() {
        assert_eq!(local_file("https://example.invalid/cover.jpg"), None);
    }

    #[test]
    fn the_album_names_the_file_and_the_title_fills_in() {
        let (_, album) = key(&track("Radiohead", Some("OK Computer"), "Let Down")).expect("a key");
        assert_eq!(album, "ok computer");

        let (_, single) = key(&track("Radiohead", None, "Creep")).expect("a key");
        assert_eq!(single, "creep");
    }

    #[test]
    fn a_track_with_no_artist_has_nothing_to_look_up() {
        assert_eq!(key(&Track::default()), None);
    }

    #[test]
    fn every_track_of_one_record_shares_a_cover() {
        let first = key(&track("Radiohead", Some("OK Computer"), "Airbag"));
        let second = key(&track("Radiohead", Some("OK Computer"), "Karma Police"));
        assert_eq!(first, second);
    }
}
