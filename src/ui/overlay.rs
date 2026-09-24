//! The line of lyrics that sits above everything else.
//!
//! A plain `gtk::Window`, never libadwaita: its background is opaque and
//! fights the transparency the whole overlay depends on.

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider, Label};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::error::Error;

const STYLE: &str = "
window { background: transparent; }
label {
    color: white;
    font-size: 30px;
    font-weight: 600;
    text-shadow: 0 2px 6px rgba(0, 0, 0, 0.9);
    padding: 0 24px;
}
";

pub struct Overlay {
    label: Label,
}

impl Overlay {
    /// Builds the window and puts it on screen.
    pub fn build(app: &Application) -> Result<Self, Error> {
        let display = gtk::gdk::Display::default().ok_or(Error::NoDisplay)?;

        let provider = CssProvider::new();
        provider.load_from_string(STYLE);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let label = Label::builder()
            .justify(gtk::Justification::Center)
            .wrap(true)
            .opacity(0.0)
            .build();

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(900)
            .default_height(120)
            .child(&label)
            .build();

        window.init_layer_shell();
        // Top does not survive a fullscreen window; Overlay does, and that is
        // the whole point of the project.
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Bottom, true);
        window.set_margin(Edge::Bottom, 100);
        // Without this the compositor shrinks every other window, the way it
        // does for a panel.
        window.set_exclusive_zone(-1);
        window.present();

        Ok(Self { label })
    }

    /// Shows a line, or nothing at all during an instrumental.
    pub fn show(&self, line: Option<&str>) {
        self.label.set_text(line.unwrap_or_default());
        self.label
            .set_opacity(if line.is_some() { 1.0 } else { 0.0 });
    }
}
