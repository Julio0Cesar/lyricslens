//! Everything that talks to a player over D-Bus: finding one, and following
//! what it plays.

use std::collections::HashMap;

use futures_util::StreamExt;
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::fdo::PropertiesProxy;
use zbus::names::{InterfaceName, OwnedBusName};
use zbus::proxy;
use zbus::zvariant::OwnedValue;

use super::{Event, Track};
use crate::error::Error;

/// Every MPRIS player owns a bus name starting with this.
const PREFIX: &str = "org.mpris.MediaPlayer2.";

/// Only the parts of the player interface this milestone needs.
#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub trait Player {
    #[zbus(property)]
    fn metadata(
        &self,
    ) -> zbus::Result<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>;

    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;
}

/// Bus names of every running MPRIS player, in the order the bus returns them.
pub async fn list(connection: &Connection) -> Result<Vec<OwnedBusName>, Error> {
    let bus = DBusProxy::new(connection).await?;
    let mut players: Vec<OwnedBusName> = bus
        .list_names()
        .await?
        .into_iter()
        .filter(|name| name.starts_with(PREFIX))
        .collect();
    // The bus order is arbitrary; sorting makes the pick reproducible when
    // nothing is playing.
    players.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    Ok(players)
}

/// Picks the player to follow: the one that is actually playing, else the first.
///
/// Asking each player for its status costs one round-trip per player, which is
/// fine because this runs when the overlay starts, not on every track.
pub async fn pick(connection: &Connection) -> Result<Option<OwnedBusName>, Error> {
    let players = list(connection).await?;
    for name in &players {
        let proxy = PlayerProxy::builder(connection)
            .destination(name.clone())?
            .build()
            .await?;
        if proxy.playback_status().await.as_deref() == Ok("Playing") {
            return Ok(Some(name.clone()));
        }
    }
    Ok(players.into_iter().next())
}

/// A name like `org.mpris.MediaPlayer2.firefox.instance_1_1` shortened to `firefox`.
pub fn short_name(name: &OwnedBusName) -> &str {
    name.as_str()
        .strip_prefix(PREFIX)
        .unwrap_or(name.as_str())
        .split('.')
        .next()
        .unwrap_or(name.as_str())
}

const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

/// Reads the current track, then forwards every later change until the channel
/// closes or the player leaves the bus.
///
/// There is no polling loop: zbus keeps a property cache fed by the signal, so
/// reading `Metadata` after a change costs nothing on the bus.
pub async fn follow(
    connection: &Connection,
    name: &OwnedBusName,
    events: async_channel::Sender<Event>,
) -> Result<(), Error> {
    // Subscribing before the first read, not after: a song that changes in
    // between would be missed, and every later change would be compared against
    // a track that was already stale.
    let properties = PropertiesProxy::builder(connection)
        .destination(name.clone())?
        .path("/org/mpris/MediaPlayer2")?
        .build()
        .await?;
    let interface = InterfaceName::try_from(PLAYER_INTERFACE)?;
    let mut changes = properties.receive_properties_changed().await?;

    let player = PlayerProxy::builder(connection)
        .destination(name.clone())?
        .build()
        .await?;

    let mut last = Track::from_metadata(&player.metadata().await?);
    tracing::info!(player = short_name(name), track = ?last, "following");
    if events
        .send(Event::TrackChanged(last.clone()))
        .await
        .is_err()
    {
        return Ok(());
    }

    while let Some(change) = changes.next().await {
        let args = change.args()?;
        tracing::debug!(
            interface = %args.interface_name,
            changed = ?args.changed_properties.keys().collect::<Vec<_>>(),
            invalidated = ?args.invalidated_properties,
            "properties changed"
        );
        if args.interface_name != interface {
            continue;
        }
        let Some(metadata) = args.changed_properties.get("Metadata") else {
            // Position and volume also arrive here; only Metadata matters now.
            continue;
        };
        let metadata: HashMap<String, OwnedValue> =
            match metadata.try_clone().and_then(TryInto::try_into) {
                Ok(metadata) => metadata,
                Err(error) => {
                    tracing::warn!(%error, "Metadata arrived with an unexpected type");
                    continue;
                }
            };

        let track = Track::from_metadata(&metadata);
        if track == last {
            continue;
        }
        last = track.clone();
        tracing::info!(track = ?track, "track changed");
        if events.send(Event::TrackChanged(track)).await.is_err() {
            break;
        }
    }

    tracing::info!(player = short_name(name), "player left the bus");
    Ok(())
}
