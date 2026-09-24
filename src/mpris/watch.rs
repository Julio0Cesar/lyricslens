//! Following one player's track changes through `PropertiesChanged`.

use std::collections::HashMap;

use futures_util::StreamExt;
use zbus::Connection;
use zbus::fdo::PropertiesProxy;
use zbus::names::{InterfaceName, OwnedBusName};
use zbus::zvariant::OwnedValue;

use super::player::{PlayerProxy, short_name};
use super::track::Track;
use crate::error::Error;

const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

/// What the overlay reacts to.
#[derive(Debug, Clone)]
pub enum Event {
    TrackChanged(Track),
}

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
