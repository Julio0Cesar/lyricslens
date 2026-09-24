//! What is playing, and where that knowledge comes from.
//!
//! The bridge between the D-Bus reader and the GTK loop lives here too.
//!
//! zbus runs on tokio and GTK has its own main loop, and neither can drive the
//! other. So tokio lives on a thread of its own and the two sides only meet on
//! a channel that both know how to await.

use std::collections::HashMap;
use std::time::Duration;

use zbus::Connection;
use zbus::zvariant::{OwnedValue, Value};

pub mod mpris;

/// How long to wait before looking for a player again.
const RETRY: Duration = Duration::from_secs(2);

/// Starts the reader on its own thread and hands back the receiving end.
///
/// The channel is bounded: if the interface ever stops reading, the reader
/// waits instead of growing a queue of stale tracks.
pub fn start() -> async_channel::Receiver<Event> {
    let (sender, receiver) = async_channel::bounded(8);

    std::thread::Builder::new()
        .name("mpris".to_owned())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::error!(%error, "could not start the D-Bus runtime");
                    return;
                }
            };
            runtime.block_on(run(sender));
        })
        .expect("spawning a thread");

    receiver
}

/// Follows whatever is playing, and keeps looking when nothing is.
async fn run(sender: async_channel::Sender<Event>) {
    loop {
        if sender.is_closed() {
            return;
        }
        if let Err(error) = once(&sender).await {
            tracing::warn!(%error, "lost the player");
        }
        tokio::time::sleep(RETRY).await;
    }
}

async fn once(sender: &async_channel::Sender<Event>) -> Result<(), crate::error::Error> {
    let connection = Connection::session().await?;
    let Some(name) = mpris::pick(&connection).await? else {
        tracing::debug!("no MPRIS player on the bus");
        return Ok(());
    };
    mpris::follow(&connection, &name, sender.clone()).await
}

/// What the overlay reacts to.
#[derive(Debug, Clone)]
pub enum Event {
    TrackChanged(Track),
}

/// What is playing, as far as the player is willing to say.
///
/// Every field is optional on purpose. The MPRIS spec makes all of them
/// optional, and players disagree on which ones they send: a browser playing a
/// video often has a title and nothing else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Track {
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub length: Option<Duration>,
}

impl Track {
    /// Reads the four fields the overlay needs out of `org.mpris.MediaPlayer2.Player.Metadata`.
    ///
    /// Anything missing, of the wrong type, or empty becomes `None` rather than
    /// an error: a player sending garbage should cost us one blank line, not a
    /// crash.
    pub fn from_metadata(metadata: &HashMap<String, OwnedValue>) -> Self {
        Self {
            title: text(metadata, "xesam:title"),
            artists: text_list(metadata, "xesam:artist"),
            album: text(metadata, "xesam:album"),
            length: metadata
                .get("mpris:length")
                .and_then(microseconds)
                .map(Duration::from_micros),
        }
    }

    /// True when the player gave us nothing worth showing.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.artists.is_empty() && self.album.is_none()
    }
}

fn text(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let value = metadata.get(key)?;
    let text = match value.downcast_ref::<&str>() {
        Ok(text) => text.to_owned(),
        // Some players send the title as a one-element array.
        Err(_) => text_list(metadata, key).into_iter().next()?,
    };
    (!text.trim().is_empty()).then_some(text)
}

fn text_list(metadata: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    let Some(value) = metadata.get(key) else {
        return Vec::new();
    };
    if let Ok(text) = value.downcast_ref::<&str>() {
        return vec![text.to_owned()];
    }
    let Ok(array) = value.downcast_ref::<&zbus::zvariant::Array>() else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|item| match item {
            Value::Str(text) => Some(text.to_string()),
            _ => None,
        })
        .filter(|text| !text.trim().is_empty())
        .collect()
}

/// `mpris:length` is microseconds, but players send it as any integer width
/// they feel like, and a negative one means "unknown".
fn microseconds(value: &OwnedValue) -> Option<u64> {
    if let Ok(n) = value.downcast_ref::<u64>() {
        return Some(n);
    }
    if let Ok(n) = value.downcast_ref::<i64>() {
        return u64::try_from(n).ok();
    }
    if let Ok(n) = value.downcast_ref::<i32>() {
        return u64::try_from(n).ok();
    }
    if let Ok(n) = value.downcast_ref::<u32>() {
        return Some(u64::from(n));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict(pairs: Vec<(&str, Value<'static>)>) -> HashMap<String, OwnedValue> {
        pairs
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_owned(),
                    OwnedValue::try_from(value).expect("value is convertible"),
                )
            })
            .collect()
    }

    #[test]
    fn reads_a_well_formed_dict() {
        let track = Track::from_metadata(&dict(vec![
            ("xesam:title", Value::from("Paranoid Android")),
            (
                "xesam:artist",
                Value::from(vec!["Radiohead".to_owned(), "Nigel Godrich".to_owned()]),
            ),
            ("xesam:album", Value::from("OK Computer")),
            ("mpris:length", Value::from(383_000_000u64)),
        ]));

        assert_eq!(track.title.as_deref(), Some("Paranoid Android"));
        assert_eq!(track.artists, ["Radiohead", "Nigel Godrich"]);
        assert_eq!(track.album.as_deref(), Some("OK Computer"));
        assert_eq!(track.length, Some(Duration::from_secs(383)));
    }

    #[test]
    fn an_empty_dict_is_an_empty_track_not_a_panic() {
        let track = Track::from_metadata(&HashMap::new());
        assert_eq!(track, Track::default());
        assert!(track.is_empty());
    }

    #[test]
    fn wrong_types_are_dropped_field_by_field() {
        let track = Track::from_metadata(&dict(vec![
            ("xesam:title", Value::from("Weather Report")),
            ("xesam:artist", Value::from(42u32)),
            ("xesam:album", Value::from(vec![1u32, 2u32])),
            ("mpris:length", Value::from("not a number")),
        ]));

        assert_eq!(track.title.as_deref(), Some("Weather Report"));
        assert!(track.artists.is_empty());
        assert_eq!(track.album, None);
        assert_eq!(track.length, None);
    }

    #[test]
    fn a_negative_length_means_unknown() {
        let track = Track::from_metadata(&dict(vec![("mpris:length", Value::from(-1i64))]));
        assert_eq!(track.length, None);
    }

    #[test]
    fn blank_strings_count_as_missing() {
        let track = Track::from_metadata(&dict(vec![
            ("xesam:title", Value::from("   ")),
            ("xesam:artist", Value::from(vec!["".to_owned()])),
        ]));
        assert!(track.is_empty());
    }
}
