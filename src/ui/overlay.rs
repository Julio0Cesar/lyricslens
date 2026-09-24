//! The line of lyrics that sits above everything else.
//!
//! A plain `gtk::Window`, never libadwaita: its background is opaque and
//! fights the transparency the whole overlay depends on.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::glib::ControlFlow;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider, Label};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::error::Error;

/// Half a fade. One line goes out over this, the next comes in over it.
const FADE: Duration = Duration::from_millis(140);

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
    fade: Rc<RefCell<Fade>>,
}

/// What the animation is doing, and what it has been asked to show next.
struct Fade {
    shown: Option<String>,
    wanted: Option<String>,
    running: bool,
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

        Ok(Self {
            label,
            fade: Rc::new(RefCell::new(Fade {
                shown: None,
                wanted: None,
                running: false,
            })),
        })
    }

    /// Shows a line, or nothing at all during an instrumental.
    pub fn show(&self, line: Option<&str>) {
        let line = line.map(str::to_owned);
        {
            let mut fade = self.fade.borrow_mut();
            if fade.wanted == line || (!fade.running && fade.shown == line) {
                return;
            }
            fade.wanted = line;
            if fade.running {
                // The animation running now will pick the new text up on its way.
                return;
            }
            fade.running = true;
        }
        self.animate();
    }

    /// Fades the current line out, swaps the text, fades the next one in.
    ///
    /// Driven by the frame clock rather than a timer, so it runs at whatever
    /// rate the screen actually refreshes, and stops as soon as it is done.
    fn animate(&self) {
        let started = Instant::now();
        let fade = Rc::clone(&self.fade);
        let swapped = std::cell::Cell::new(false);

        self.label.add_tick_callback(move |label, _| {
            let elapsed = started.elapsed();

            if elapsed < FADE {
                label.set_opacity(1.0 - elapsed.as_secs_f64() / FADE.as_secs_f64());
                return ControlFlow::Continue;
            }

            if !swapped.get() {
                swapped.set(true);
                let mut state = fade.borrow_mut();
                state.shown = state.wanted.clone();
                label.set_text(state.shown.as_deref().unwrap_or_default());
            }

            // Nothing to fade in during an instrumental: the screen stays empty.
            if fade.borrow().shown.is_none() {
                fade.borrow_mut().running = false;
                label.set_opacity(0.0);
                return ControlFlow::Break;
            }

            let progress = (elapsed - FADE).as_secs_f64() / FADE.as_secs_f64();
            if progress >= 1.0 {
                label.set_opacity(1.0);
                fade.borrow_mut().running = false;
                return ControlFlow::Break;
            }
            label.set_opacity(progress);
            ControlFlow::Continue
        });
    }
}
