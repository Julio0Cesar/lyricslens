//! The bridge between the D-Bus reader and the GTK loop.
//!
//! zbus runs on tokio and GTK has its own main loop, and neither can drive the
//! other. So tokio lives on a thread of its own and the two sides only meet on
//! a channel that both know how to await.

use std::time::Duration;

use zbus::Connection;

use crate::mpris::{player, watch};

/// How long to wait before looking for a player again.
const RETRY: Duration = Duration::from_secs(2);

/// Starts the reader on its own thread and hands back the receiving end.
///
/// The channel is bounded: if the interface ever stops reading, the reader
/// waits instead of growing a queue of stale tracks.
pub fn start() -> async_channel::Receiver<watch::Event> {
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
async fn run(sender: async_channel::Sender<watch::Event>) {
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

async fn once(sender: &async_channel::Sender<watch::Event>) -> Result<(), crate::error::Error> {
    let connection = Connection::session().await?;
    let Some(name) = player::pick(&connection).await? else {
        tracing::debug!("no MPRIS player on the bus");
        return Ok(());
    };
    watch::follow(&connection, &name, sender.clone()).await
}
