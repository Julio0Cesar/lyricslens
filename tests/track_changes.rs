//! Proves the overlay hears a track change through the signal, not by polling.
//!
//! A fake player is published on the session bus, followed, and then changed.
//! Needs a bus: run with `dbus-run-session -- cargo test`.

use std::collections::HashMap;
use std::time::Duration;

use lyricslens::media::mpris::follow;
use lyricslens::media::{Event, Track};
use zbus::names::OwnedBusName;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{Connection, interface};

const NAME: &str = "org.mpris.MediaPlayer2.lyricslenstest";
const PATH: &str = "/org/mpris/MediaPlayer2";

struct FakePlayer {
    metadata: HashMap<String, OwnedValue>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl FakePlayer {
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        self.metadata.clone()
    }

    #[zbus(property)]
    fn playback_status(&self) -> String {
        "Playing".to_owned()
    }
}

fn metadata(title: &str) -> HashMap<String, OwnedValue> {
    HashMap::from([(
        "xesam:title".to_owned(),
        OwnedValue::try_from(Value::from(title)).expect("a string is convertible"),
    )])
}

#[tokio::test(flavor = "multi_thread")]
async fn a_metadata_change_reaches_the_channel() {
    let player = zbus::connection::Builder::session()
        .expect("a session bus")
        .name(NAME)
        .expect("the name is free")
        .serve_at(
            PATH,
            FakePlayer {
                metadata: metadata("first song"),
            },
        )
        .expect("serving the player")
        .build()
        .await
        .expect("publishing the player");

    let (sender, events) = async_channel::bounded(8);
    let watcher = tokio::spawn(async move {
        let connection = Connection::session().await.expect("a session bus");
        let name = OwnedBusName::try_from(NAME).expect("a valid name");
        follow(&connection, &name, sender).await
    });

    let first = next(&events).await;
    assert_eq!(first.title.as_deref(), Some("first song"));

    let interface = player
        .object_server()
        .interface::<_, FakePlayer>(PATH)
        .await
        .expect("the published interface");
    interface.get_mut().await.metadata = metadata("second song");
    interface
        .get_mut()
        .await
        .metadata_changed(interface.signal_emitter())
        .await
        .expect("emitting PropertiesChanged");

    let second = next(&events).await;
    assert_eq!(second.title.as_deref(), Some("second song"));

    watcher.abort();
}

/// Waits for one track, with the same second the milestone promises.
async fn next(events: &async_channel::Receiver<Event>) -> Track {
    let event = tokio::time::timeout(Duration::from_secs(1), events.recv())
        .await
        .expect("a track within a second")
        .expect("the channel is open");
    match event {
        Event::TrackChanged(track) => track,
    }
}
