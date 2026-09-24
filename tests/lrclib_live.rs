//! Hits the real LRCLIB. Ignored by default: it needs the network, and a
//! service that costs nothing should not be called on every `cargo test`.
//!
//! Run it with `cargo test -- --ignored`.

use std::time::Duration;

use lyricslens::lyrics::lrclib::Client;
use lyricslens::lyrics::normalize::clean;

#[tokio::test]
#[ignore = "needs the network"]
async fn a_real_song_comes_back_with_synced_lyrics() {
    let query = clean(None, "Radiohead - Creep (Official Video)");
    assert_eq!(query.artist.as_deref(), Some("Radiohead"));

    let lyrics = Client::new()
        .expect("a client")
        .lyrics(&query, Some(Duration::from_secs(238)))
        .await
        .expect("the service answered")
        .expect("the song is known");

    assert!(lyrics.lines.len() > 10, "got {} lines", lyrics.lines.len());
    assert!(lyrics.lines.iter().any(|line| line.sung().is_some()));
}

/// The whole chain on whatever is playing right now: bus, cleanup, service.
/// Prints what it found, because a song with no lyrics on LRCLIB is a valid
/// answer and should not fail the run.
#[tokio::test]
#[ignore = "needs the network and something playing"]
async fn whatever_is_playing_goes_through_the_whole_chain() {
    use lyricslens::lyrics::normalize::from_track;
    use lyricslens::media::{Event, mpris};
    use zbus::Connection;

    let connection = Connection::session().await.expect("a session bus");
    let Some(name) = mpris::pick(&connection).await.expect("the bus answered") else {
        eprintln!("no player running");
        return;
    };

    let (sender, events) = async_channel::bounded(1);
    let follower = tokio::spawn(async move {
        let connection = Connection::session().await.expect("a session bus");
        let _ = mpris::follow(&connection, &name, sender).await;
    });

    let Event::TrackChanged(track) = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("a track within five seconds")
        .expect("the channel is open");
    follower.abort();

    let query = from_track(&track);
    eprintln!("playing: {track:?}\nasking for: {query:?}");

    match Client::new()
        .expect("a client")
        .lyrics(&query, track.length)
        .await
    {
        Ok(Some(lyrics)) => eprintln!("{} synced lines", lyrics.lines.len()),
        Ok(None) => eprintln!("no synced lyrics for this one"),
        // A video that is not a song is the common case here; the point of this
        // check is that the chain runs, not that the service knows the track.
        Err(error) => eprintln!("the service refused: {error}"),
    }
}
