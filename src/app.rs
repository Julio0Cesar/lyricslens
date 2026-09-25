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
use crate::lyrics::lrclib::{Candidate, Client};
use crate::lyrics::normalize::{Query, from_track};
use crate::lyrics::{Lyrics, lrc};
use crate::media::{Event, Track, mpris};
use crate::store::{cache, settings::Settings};

/// How long to wait before looking for a player again.
const RETRY: Duration = Duration::from_secs(2);

/// Names the player to follow, over whatever the settings say.
const OVERRIDE: &str = "LYRICSLENS_PLAYER";

/// What the interface asks the worker to do.
///
/// Only the worker has a runtime to reach the network with, so a search
/// started from the preferences window is a message rather than a call.
#[derive(Debug, Clone)]
pub enum Request {
    /// Every recording under this name, for choosing by hand.
    Search { artist: String, title: String },
    /// Use these lyrics for the track playing now, and remember the choice.
    Choose(Box<Candidate>),
}

/// Everything the interface needs to know.
#[derive(Debug, Clone)]
pub enum Update {
    Media(Event),
    /// Which player is being followed, by bus name. The manual offset is kept
    /// per player, so the interface needs to know whose it is.
    Player(String),
    /// The lyrics for the track playing now, or nothing found.
    ///
    /// Boxed because it dwarfs every other variant, and a whole song would
    /// otherwise set the size of the channel's every message.
    Lyrics(Box<Option<Lyrics>>),
    /// The answer to a search, in the order the service returned it.
    Candidates(Vec<Candidate>),
    /// A release newer than this build exists. Nothing is installed by it.
    NewVersion(String),
}

/// Starts the worker and hands back the receiving end.
///
/// The channel is bounded: if the interface ever stops reading, the worker
/// waits instead of growing a queue of stale updates.
pub fn start(
    settings: Settings,
) -> (
    async_channel::Receiver<Update>,
    async_channel::Sender<Request>,
) {
    let (sender, receiver) = async_channel::bounded(64);
    let (requests, incoming) = async_channel::bounded(8);

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
            runtime.block_on(run(sender, incoming, settings));
        })
        .expect("spawning a thread");

    (receiver, requests)
}

/// Follows whatever is playing, and keeps looking when nothing is.
async fn run(
    updates: async_channel::Sender<Update>,
    requests: async_channel::Receiver<Request>,
    settings: Settings,
) {
    if let Ok(client) = Client::new() {
        let updates = updates.clone();
        tokio::spawn(async move {
            if let Some(version) = client.check_for_a_newer_release().await {
                tracing::info!(version, "a newer release is out");
                let _ = updates.send(Update::NewVersion(version)).await;
            }
        });
    }

    // The variable wins over the file: it is how a single run is pointed at a
    // different player without editing anything.
    let wanted = std::env::var(OVERRIDE)
        .ok()
        .filter(|wanted| !wanted.is_empty())
        .or(settings.player);

    loop {
        if updates.is_closed() {
            return;
        }
        if let Err(error) = once(&updates, &requests, wanted.as_deref()).await {
            tracing::warn!(%error, "lost the player");
        }
        tokio::time::sleep(RETRY).await;
    }
}

async fn once(
    updates: &async_channel::Sender<Update>,
    requests: &async_channel::Receiver<Request>,
    wanted: Option<&str>,
) -> Result<(), Error> {
    let connection = Connection::session().await?;
    let Some(name) = mpris::pick(&connection, wanted).await? else {
        tracing::debug!("no MPRIS player on the bus");
        // Whatever was on screen belongs to a player that is gone.
        let _ = updates
            .send(Update::Media(Event::TrackChanged(Track::default())))
            .await;
        return Ok(());
    };
    if updates
        .send(Update::Player(name.as_str().to_owned()))
        .await
        .is_err()
    {
        return Ok(());
    }

    let (events, incoming) = async_channel::bounded(64);
    let follower = {
        let connection = connection.clone();
        let name = name.clone();
        tokio::spawn(async move { mpris::follow(&connection, &name, events).await })
    };

    let client = Client::new()?;
    let mut lookup: Option<JoinHandle<()>> = None;
    let mut playing: Option<Track> = None;

    loop {
        let event = tokio::select! {
            event = incoming.recv() => match event {
                Ok(event) => event,
                Err(_) => break,
            },
            request = requests.recv() => {
                match request {
                    Ok(request) => {
                        answer(&client, request, playing.clone(), updates).await;
                        continue;
                    }
                    Err(_) => break,
                }
            }
        };

        if let Event::TrackChanged(track) = &event {
            playing = Some(track.clone());
            // A lookup for the previous song is worthless now, and its answer
            // arriving late would put the wrong lyrics on screen.
            if let Some(lookup) = lookup.take() {
                lookup.abort();
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
    // The player left, or stopped talking. Either way there is nothing to sing
    // along to, and the last line must not sit there as if there were.
    let _ = updates
        .send(Update::Media(Event::TrackChanged(Track::default())))
        .await;
    Ok(())
}

/// Handles what the preferences window asked for.
async fn answer(
    client: &Client,
    request: Request,
    playing: Option<Track>,
    updates: &async_channel::Sender<Update>,
) {
    match request {
        Request::Search { artist, title } => {
            let found = match client.search(&artist, &title).await {
                Ok(found) => found,
                Err(error) => {
                    tracing::warn!(%error, "the search failed");
                    Vec::new()
                }
            };
            tracing::info!(results = found.len(), artist, title, "searched by hand");
            let _ = updates.send(Update::Candidates(found)).await;
        }
        Request::Choose(chosen) => {
            let lyrics = lrc::parse(&chosen.lrc);
            // Cached under what is playing, not under what was searched for:
            // the point is that the same track comes back right next time.
            if let Some(track) = playing {
                let query = from_track(&track);
                cache::put(
                    query.artist.as_deref().unwrap_or_default(),
                    &query.title,
                    track.length,
                    &chosen.lrc,
                );
            }
            tracing::info!(lines = lyrics.lines.len(), "lyrics chosen by hand");
            let _ = updates.send(Update::Lyrics(Box::new(Some(lyrics)))).await;
        }
    }
}

async fn fetch(client: Client, track: Track, updates: async_channel::Sender<Update>) {
    let query = from_track(&track);
    if query.title.is_empty() {
        let _ = updates.send(Update::Lyrics(Box::new(None))).await;
        return;
    }

    let artist = query.artist.as_deref().unwrap_or_default();
    if let Some(lyrics) = cache::get(artist, &query.title, track.length) {
        tracing::info!(lines = lyrics.lines.len(), ?query, "lyrics from the cache");
        let _ = updates.send(Update::Lyrics(Box::new(Some(lyrics)))).await;
        return;
    }

    let found = match client.lyrics(&query, track.length).await {
        Ok(found) => found,
        Err(error) => {
            tracing::warn!(%error, ?query, "could not fetch lyrics");
            let _ = updates.send(Update::Lyrics(Box::new(None))).await;
            return;
        }
    };

    let lyrics = found.map(|found| {
        cache::put(artist, &query.title, track.length, &found.lrc);
        found.lyrics
    });
    report(&query, lyrics.as_ref());
    let _ = updates.send(Update::Lyrics(Box::new(lyrics))).await;
}

fn report(query: &Query, lyrics: Option<&Lyrics>) {
    match lyrics {
        Some(lyrics) => tracing::info!(lines = lyrics.lines.len(), ?query, "lyrics found"),
        None => tracing::info!(?query, "no synced lyrics for this one"),
    }
}
