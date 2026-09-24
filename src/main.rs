use std::time::{Duration, Instant};

use gtk::gdk::Display;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider, Label};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use lyricslens::error;
use lyricslens::media;
use lyricslens::media::{Event, Track};
use lyricslens::sync::clock::Clock;

fn main() -> gtk::glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let events = media::start();

    let app = Application::builder()
        .application_id("io.github.julio0cesar.lyricslens")
        .build();

    app.connect_activate(move |app| {
        let Some(display) = Display::default() else {
            tracing::error!("{}", error::Error::NoDisplay);
            return;
        };

        let provider = CssProvider::new();
        provider.load_from_string(
            "window { background: transparent; }
             label { color: white; font-size: 28px; }",
        );
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let label = Label::new(Some("waiting for a player"));

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(600)
            .default_height(200)
            .child(&label)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Bottom, true);
        window.set_margin(Edge::Bottom, 100);
        window.set_exclusive_zone(-1);

        window.present();

        // Until there are lyrics to show, the overlay displays the track and
        // where the clock thinks it is. That is what proves both halves work.
        let events = events.clone();
        gtk::glib::spawn_future_local(async move {
            let mut clock = Clock::new(0);
            let mut track = Track::default();
            let mut stalled = false;

            while let Ok(event) = events.recv().await {
                match event {
                    Event::TrackChanged(next) => {
                        clock.reset();
                        stalled = false;
                        track = next;
                    }
                    Event::Playback(state) => {
                        stalled = false;
                        clock.playback(state, Instant::now());
                    }
                    Event::PositionStalled => stalled = true,
                    Event::Position { reading, at } => {
                        clock.sample(reading, at);
                    }
                }
                let position = if stalled {
                    None
                } else {
                    clock.position(Instant::now())
                };
                label.set_text(&describe(&track, position, stalled));
            }
        });
    });

    app.run()
}

fn describe(track: &Track, position: Option<Duration>, stalled: bool) -> String {
    if track.is_empty() {
        return "nothing playing".to_owned();
    }

    let title = track.title.as_deref().unwrap_or("unknown track");
    let mut line = if track.artists.is_empty() {
        title.to_owned()
    } else {
        format!("{} — {}", track.artists.join(", "), title)
    };
    if let Some(position) = position {
        let seconds = position.as_secs();
        line.push_str(&format!("  ·  {}:{:02}", seconds / 60, seconds % 60));
    } else if stalled {
        line.push_str("  ·  this player does not report its position");
    }
    line
}
