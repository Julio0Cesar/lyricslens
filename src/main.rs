use gtk4 as gtk;
use gtk::prelude::*;
use gtk::gdk::Display;
use gtk::{Application, ApplicationWindow, CssProvider, Label};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

fn main() -> gtk::glib::ExitCode {
    let app = Application::builder()
        .application_id("io.github.julio0cesar.lyricslens")
        .build();

    app.connect_activate(|app| {
        let provider = CssProvider::new();
        provider.load_from_string(
            "window { background: transparent; }
             label { color: white; font-size: 28px; }",
        );
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("no display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let label = Label::new(Some("hello from a layer surface"));

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
    });

    app.run()
}