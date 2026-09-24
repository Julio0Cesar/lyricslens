//! Measures how far the clock drifts from a player that only reports whole
//! seconds — which is what a browser does, and the case the whole anchoring
//! scheme exists for.
//!
//! The player here is a fake one on the session bus, so the measurement is the
//! same on any machine. Needs a bus: `dbus-run-session -- cargo test`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use lyricslens::media::{Event, mpris};
use lyricslens::sync::Playback;
use lyricslens::sync::clock::{Clock, Reading};
use zbus::names::OwnedBusName;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{Connection, interface};

const NAME: &str = "org.mpris.MediaPlayer2.lyricslensdrift";
const PATH: &str = "/org/mpris/MediaPlayer2";

/// Long enough for a dozen edges, short enough to sit in CI.
const RUN: Duration = Duration::from_secs(15);

/// The poll interval is 50ms, so the edge can be noticed that late. The rest is
/// scheduling noise on a loaded machine.
const ALLOWED: Duration = Duration::from_millis(150);

struct FakePlayer {
    started: Instant,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl FakePlayer {
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        HashMap::from([(
            "xesam:title".to_owned(),
            OwnedValue::try_from(Value::from("a song")).expect("a string is convertible"),
        )])
    }

    #[zbus(property)]
    fn playback_status(&self) -> String {
        "Playing".to_owned()
    }

    /// Microseconds, rounded down to the second, exactly as a browser reports.
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 {
        i64::try_from(self.started.elapsed().as_secs()).unwrap_or(0) * 1_000_000
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_clock_stays_within_a_fraction_of_a_second() {
    let truth = Instant::now();
    let _player = zbus::connection::Builder::session()
        .expect("a session bus")
        .name(NAME)
        .expect("the name is free")
        .serve_at(PATH, FakePlayer { started: truth })
        .expect("serving the player")
        .build()
        .await
        .expect("publishing the player");

    let (sender, events) = async_channel::bounded(64);
    let follower = tokio::spawn(async move {
        let connection = Connection::session().await.expect("a session bus");
        let name = OwnedBusName::try_from(NAME).expect("a valid name");
        let _ = mpris::follow(&connection, &name, sender).await;
    });

    let mut clock = Clock::new(0);
    let mut worst = Duration::ZERO;
    let mut total = Duration::ZERO;
    let mut edges = 0u32;
    // How far the estimate sits from where the song really is. The edge can
    // only be noticed one poll late, so this is the number that matters.
    let mut worst_absolute = Duration::ZERO;
    let started = Instant::now();

    while started.elapsed() < RUN {
        let Ok(Ok(event)) = tokio::time::timeout(Duration::from_secs(2), events.recv()).await
        else {
            break;
        };

        match event {
            Event::TrackChanged(_) => clock.reset(),
            Event::PositionStalled => panic!("the fake player does report its position"),
            Event::Playback(state) => clock.playback(state, Instant::now()),
            Event::Position { reading, at } => {
                // The estimate taken before the edge re-anchors is the error
                // that piled up since the previous one.
                let estimate = clock.position(at);
                if let Some(estimate) = estimate {
                    worst_absolute = worst_absolute.max(estimate.abs_diff(at - truth));
                }
                if clock.sample(reading, at) == Reading::Edge
                    && let Some(estimate) = estimate
                {
                    let error = estimate.abs_diff(reading);
                    worst = worst.max(error);
                    total += error;
                    edges += 1;
                }
            }
        }
    }
    follower.abort();

    assert!(edges >= 10, "only {edges} edges in {RUN:?}");
    eprintln!(
        "{edges} edges · between anchors: mean {:?}, worst {worst:?} ·          against the song: worst {worst_absolute:?}",
        total / edges
    );
    assert!(worst < ALLOWED, "drift between anchors was {worst:?}");
    assert!(
        worst_absolute < ALLOWED,
        "the estimate sat {worst_absolute:?} away from the song"
    );
}

/// The player stops moving and the estimate has to stop with it.
#[tokio::test(flavor = "multi_thread")]
async fn a_pause_does_not_let_the_estimate_run_on() {
    let mut clock = Clock::new(0);
    let base = Instant::now();

    clock.playback(Playback::Playing, base);
    clock.sample(Duration::from_secs(30), base);
    clock.playback(Playback::Paused, base + Duration::from_millis(250));

    let frozen = clock.position(base + Duration::from_millis(250));
    assert_eq!(frozen, Some(Duration::from_millis(30_250)));
    assert_eq!(clock.position(base + Duration::from_secs(20)), frozen);
}
