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
use crate::store::settings::Settings;

mod x11;

/// Half a fade. One line goes out over this, the next comes in over it.
const FADE: Duration = Duration::from_millis(140);

/// Marks the window while it is being moved, so there is something to grab.
const POSITIONING: &str = "positioning";

fn style(font_size: u32) -> String {
    format!(
        "window {{ background: transparent; }}
         window.{POSITIONING} {{
             background: rgba(0, 0, 0, 0.45);
             border: 2px dashed rgba(255, 255, 255, 0.75);
             border-radius: 12px;
         }}
         label {{
             color: white;
             font-size: {font_size}px;
             font-weight: 600;
             text-shadow: 0 2px 6px rgba(0, 0, 0, 0.9);
             padding: 0 24px;
         }}"
    )
}

#[derive(Clone)]
pub struct Overlay {
    window: ApplicationWindow,
    label: Label,
    fade: Rc<RefCell<Fade>>,
    placement: Rc<RefCell<Placement>>,
    /// False where the compositor has no layer-shell and the window fell back
    /// to X11. Every placement decision differs between the two.
    layer_shell: bool,
}

/// What the animation is doing, and what it has been asked to show next.
struct Fade {
    shown: Option<String>,
    wanted: Option<String>,
    running: bool,
}

/// Where the overlay sits, and whether it is being moved right now.
struct Placement {
    settings: Settings,
    /// Margins at the moment a drag started, so the drag is relative to them.
    grabbed: Option<(i32, i32)>,
    positioning: bool,
}

impl Overlay {
    /// Builds the window and puts it on screen.
    pub fn build(app: &Application, settings: &Settings) -> Result<Self, Error> {
        let display = gtk::gdk::Display::default().ok_or(Error::NoDisplay)?;

        let provider = CssProvider::new();
        provider.load_from_string(&style(settings.font_size));
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

        // The branch is on the protocol, never on which desktop is running.
        // GNOME is the case that lands here, because Mutter does not implement
        // wlr-layer-shell and does not intend to — but so does any plain X11
        // session.
        let layer_shell = gtk4_layer_shell::is_supported();
        if layer_shell {
            window.init_layer_shell();
            // Top does not survive a fullscreen window; Overlay does, and that
            // is the whole point of the project.
            window.set_layer(Layer::Overlay);
            // Without this the compositor shrinks every other window, the way
            // it does for a panel.
            window.set_exclusive_zone(-1);
        } else {
            tracing::info!("no layer-shell here; falling back to an X11 surface");
            window.set_decorated(false);
            x11::keep_above(&window, settings.bottom_margin);
        }

        let overlay = Self {
            window,
            label,
            fade: Rc::new(RefCell::new(Fade {
                shown: None,
                wanted: None,
                running: false,
            })),
            placement: Rc::new(RefCell::new(Placement {
                settings: settings.clone(),
                grabbed: None,
                positioning: false,
            })),
            layer_shell,
        };

        overlay.place();
        overlay.watch_drag();
        overlay.window.present();
        // Only once the surface exists is there an input region to empty.
        overlay.set_click_through(true);

        Ok(overlay)
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

    /// Brings the overlay back to the screen, which is what a second launch
    /// of the program should do instead of drawing another one.
    pub fn present(&self) {
        self.window.set_visible(true);
        self.window.present();
        self.set_click_through(!self.placement.borrow().positioning);
    }

    /// Hides the overlay, or brings it back.
    pub fn toggle(&self) {
        let visible = self.window.is_visible();
        self.window.set_visible(!visible);
        if !visible {
            self.set_click_through(!self.placement.borrow().positioning);
        }
    }

    /// Enters or leaves the mode where the overlay can be dragged.
    ///
    /// A layer surface cannot be moved by the pointer: its position comes from
    /// an anchor and a margin. So the mode takes the clicks the overlay
    /// normally lets through, turns the drag into margins, and writes them
    /// down on the way out.
    pub fn toggle_positioning(&self) {
        let positioning = {
            let mut placement = self.placement.borrow_mut();
            placement.positioning = !placement.positioning;
            placement.positioning
        };

        if positioning {
            self.window.add_css_class(POSITIONING);
            self.window.set_visible(true);
            self.label.set_opacity(1.0);
            if self.label.text().is_empty() {
                self.label.set_text("drag me");
            }
        } else {
            self.window.remove_css_class(POSITIONING);
            let placement = self.placement.borrow();
            if let Err(error) = placement.settings.save() {
                tracing::warn!(%error, "could not save where the overlay was left");
            } else {
                tracing::info!(
                    bottom = placement.settings.bottom_margin,
                    left = ?placement.settings.left_margin,
                    "overlay position saved"
                );
            }
        }
        self.set_click_through(!positioning);
    }

    /// Lets the pointer through to whatever is underneath, or takes it.
    fn set_click_through(&self, through: bool) {
        let Some(surface) = self.window.surface() else {
            return;
        };
        if through {
            // An empty region means the surface wants no pointer events at all.
            surface.set_input_region(Some(&gtk::cairo::Region::create()));
        } else {
            let (width, height) = (self.window.width(), self.window.height());
            let whole = gtk::cairo::RectangleInt::new(0, 0, width.max(1), height.max(1));
            surface.set_input_region(Some(&gtk::cairo::Region::create_rectangle(&whole)));
        }
    }

    /// Turns a drag into margins, live, while the positioning mode is on.
    fn watch_drag(&self) {
        let drag = gtk::GestureDrag::new();

        drag.connect_drag_begin({
            let overlay = self.clone();
            move |_, _, _| {
                let mut placement = overlay.placement.borrow_mut();
                if !placement.positioning {
                    return;
                }
                let left = placement
                    .settings
                    .left_margin
                    .unwrap_or_else(|| overlay.centred_left());
                placement.grabbed = Some((left, placement.settings.bottom_margin));
            }
        });

        drag.connect_drag_update({
            let overlay = self.clone();
            move |_, x, y| {
                let moved = {
                    let mut placement = overlay.placement.borrow_mut();
                    let Some((left, bottom)) = placement.grabbed else {
                        return;
                    };
                    // Dragging down moves the window down, which is a smaller
                    // distance from the bottom.
                    placement.settings.left_margin = Some((left + x as i32).max(0));
                    placement.settings.bottom_margin = (bottom - y as i32).max(0);
                    true
                };
                if moved {
                    overlay.place();
                }
            }
        });

        drag.connect_drag_end({
            let overlay = self.clone();
            move |_, _, _| {
                overlay.placement.borrow_mut().grabbed = None;
            }
        });

        self.window.add_controller(drag);
    }

    /// Puts the window where the settings say.
    fn place(&self) {
        let placement = self.placement.borrow();
        let (left, bottom) = (
            placement.settings.left_margin,
            placement.settings.bottom_margin,
        );
        drop(placement);

        if self.layer_shell {
            self.window.set_anchor(Edge::Bottom, true);
            self.window.set_margin(Edge::Bottom, bottom);
            // Anchoring left is what makes the left margin mean anything; with
            // no anchor the compositor centres the surface.
            self.window.set_anchor(Edge::Left, left.is_some());
            self.window.set_margin(Edge::Left, left.unwrap_or(0));
        } else {
            x11::place(&self.window, left, bottom);
        }
    }

    /// Where the window sits when it is centred, so a drag can start from there.
    fn centred_left(&self) -> i32 {
        let Some(surface) = self.window.surface() else {
            return 0;
        };
        let Some(monitor) = surface.display().monitor_at_surface(&surface) else {
            return 0;
        };
        (monitor.geometry().width() - self.window.width()) / 2
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
