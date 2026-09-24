//! A player on the session bus that reports a real song and a position that
//! actually moves, for working on the overlay without a music player installed.
//!
//! ```sh
//! cargo run --example fake_player -- "Radiohead" "Creep" 238
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

use zbus::interface;
use zbus::zvariant::{OwnedValue, Value};

struct Player {
    started: Instant,
    artist: String,
    title: String,
    length: Duration,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let owned = |value: Value<'_>| OwnedValue::try_from(value).expect("a convertible value");
        HashMap::from([
            ("xesam:title".to_owned(), owned(Value::from(&self.title))),
            (
                "xesam:artist".to_owned(),
                owned(Value::from(vec![self.artist.clone()])),
            ),
            (
                "mpris:length".to_owned(),
                owned(Value::from(self.length.as_micros() as u64)),
            ),
        ])
    }

    #[zbus(property)]
    fn playback_status(&self) -> String {
        "Playing".to_owned()
    }

    /// Whole seconds, in microseconds: the same shape a browser reports.
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 {
        let elapsed = self.started.elapsed().as_secs() % self.length.as_secs().max(1);
        i64::try_from(elapsed).unwrap_or(0) * 1_000_000
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let artist = args.next().unwrap_or_else(|| "Radiohead".to_owned());
    let title = args.next().unwrap_or_else(|| "Creep".to_owned());
    let length = args
        .next()
        .and_then(|seconds| seconds.parse().ok())
        .unwrap_or(238);

    let _connection = zbus::connection::Builder::session()?
        .name("org.mpris.MediaPlayer2.lyricslensfake")?
        .serve_at(
            "/org/mpris/MediaPlayer2",
            Player {
                started: Instant::now(),
                artist: artist.clone(),
                title: title.clone(),
                length: Duration::from_secs(length),
            },
        )?
        .build()
        .await?;

    println!("playing {artist} — {title} ({length}s). Ctrl-C to stop.");
    std::future::pending::<()>().await;
    Ok(())
}
