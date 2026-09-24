//! Finding the players on the session bus.

use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::names::OwnedBusName;
use zbus::proxy;

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
