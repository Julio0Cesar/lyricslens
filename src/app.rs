//! The worker thread.
//!
//! zbus and reqwest run on tokio; GTK has its own main loop, and neither can
//! drive the other. So tokio lives on a thread of its own, follows the player,
//! looks the lyrics up, and the two sides meet on a channel both know how to
//! await.

use std::time::Duration;

use tokio::task::JoinHandle;
use zbus::Connection;

use crate::error::Error;
use crate::lyrics::Lyrics;
use crate::lyrics::lrclib::Client;
use crate::lyrics::normalize::from_track;
use crate::media::{Event, Track, mpris};

/// How long to wait before looking for a player again.
const RETRY: Duration = Duration::from_secs(2);

/// Everything the interface needs to know.
#[derive(Debug, Clone)]
pub enum Update {
    Media(Event),
    /// The lyrics for the track playing now, or nothing found.
    ///
    /// Boxed because it dwarfs every other variant, and a whole song would
    /// otherwise set the size of the channel's every message.
    Lyrics(Box<Option<Lyrics>>),
}

/// Starts the worker and hands back the receiving end.
///
/// The channel is bounded: if the interface ever stops reading, the worker
/// waits instead of growing a queue of stale updates.
pub fn start() -> async_channel::Receiver<Update> {
    let (sender, receiver) = async_channel::bounded(64);

    std::thread::Builder::new()
        .name("player".to_owned())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::error!(%error, "could not start the worker runtime");
                    return;
                }
            };
            runtime.block_on(run(sender));
        })
        .expect("spawning a thread");

    receiver
}

/// Follows whatever is playing, and keeps looking when nothing is.
async fn run(updates: async_channel::Sender<Update>) {
    loop {
        if updates.is_closed() {
            return;
        }
        if let Err(error) = once(&updates).await {
            tracing::warn!(%error, "lost the player");
        }
        tokio::time::sleep(RETRY).await;
    }
}

async fn once(updates: &async_channel::Sender<Update>) -> Result<(), Error> {
    let connection = Connection::session().await?;
    let Some(name) = mpris::pick(&connection).await? else {
        tracing::debug!("no MPRIS player on the bus");
        return Ok(());
    };

    let (events, incoming) = async_channel::bounded(64);
    let follower = {
        let connection = connection.clone();
        let name = name.clone();
        tokio::spawn(async move { mpris::follow(&connection, &name, events).await })
    };

    let client = Client::new()?;
    let mut lookup: Option<JoinHandle<()>> = None;

    while let Ok(event) = incoming.recv().await {
        if let Event::TrackChanged(track) = &event {
            // A lookup for the previous song is worthless now, and its answer
            // arriving late would put the wrong lyrics on screen.
            if let Some(lookup) = lookup.take() {
                lookup.abort();
            }
            if updates.send(Update::Lyrics(Box::new(None))).await.is_err() {
                break;
            }
            lookup = Some(tokio::spawn(fetch(
                client.clone(),
                track.clone(),
                updates.clone(),
            )));
        }

        if updates.send(Update::Media(event)).await.is_err() {
            break;
        }
    }

    follower.abort();
    if let Some(lookup) = lookup {
        lookup.abort();
    }
    Ok(())
}

async fn fetch(client: Client, track: Track, updates: async_channel::Sender<Update>) {
    let query = from_track(&track);
    if query.title.is_empty() {
        return;
    }

    let found = match client.lyrics(&query, track.length).await {
        Ok(found) => found,
        Err(error) => {
            tracing::warn!(%error, ?query, "could not fetch lyrics");
            return;
        }
    };

    match &found {
        Some(lyrics) => tracing::info!(lines = lyrics.lines.len(), ?query, "lyrics found"),
        None => tracing::info!(?query, "no synced lyrics for this one"),
    }
    let _ = updates.send(Update::Lyrics(Box::new(found))).await;
}
