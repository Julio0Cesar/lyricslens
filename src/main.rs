use gtk::gdk::Display;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider, Label};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use lyricslens::error;
use lyricslens::media;
use lyricslens::mpris::track::Track;
use lyricslens::mpris::watch::Event;

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

        // Until there are lyrics to show, the overlay displays the track
        // itself: it is what proves the D-Bus side is alive.
        let events = events.clone();
        gtk::glib::spawn_future_local(async move {
            while let Ok(event) = events.recv().await {
                match event {
                    Event::TrackChanged(track) => label.set_text(&describe(&track)),
                }
            }
        });
    });

    app.run()
}

fn describe(track: &Track) -> String {
    if track.is_empty() {
        return "nothing playing".to_owned();
    }
    let title = track.title.as_deref().unwrap_or("unknown track");
    if track.artists.is_empty() {
        return title.to_owned();
    }
    format!("{} — {}", track.artists.join(", "), title)
}
