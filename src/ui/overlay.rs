//! The lyrics that sit above everything else.
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

/// Half a change. One line goes out over this, the next comes in over it.
const FADE: Duration = Duration::from_millis(160);

/// How far below its place the incoming line starts, in pixels.
const RISE: i32 = 22;

/// Marks the window while it is being moved, so there is something to grab.
const POSITIONING: &str = "positioning";

/// Everything here is scoped to this class. Without it the same rules reach
/// the preferences window, which then has no background and enormous text.
const OVERLAY: &str = "lyricslens-overlay";

fn style(settings: &Settings) -> String {
    let shadow = if settings.text_shadow {
        "text-shadow: 0 2px 6px rgba(0, 0, 0, 0.9);"
    } else {
        "text-shadow: none;"
    };
    let strip = if settings.background_opacity > 0.0 {
        format!(
            "background: rgba(0, 0, 0, {:.2}); border-radius: 14px;",
            settings.background_opacity.clamp(0.0, 1.0)
        )
    } else {
        "background: transparent;".to_owned()
    };

    format!(
        "window.{OVERLAY} {{ background: transparent; }}
         window.{OVERLAY} .lines {{ {strip} padding: 10px 24px; }}
         window.{OVERLAY}.{POSITIONING} .lines {{
             background: rgba(0, 0, 0, 0.55);
             border: 2px dashed rgba(255, 255, 255, 0.75);
             border-radius: 14px;
         }}
         window.{OVERLAY} label {{
             color: {color};
             font-size: {size}px;
             font-weight: 600;
             {shadow}
         }}
         window.{OVERLAY} label.upcoming {{
             font-size: {small}px;
             font-weight: 400;
             opacity: 0.55;
         }}
         window.{OVERLAY} label.unsung {{ opacity: 0.45; }}",
        color = settings.text_color,
        size = settings.font_size,
        small = (settings.font_size * 7 / 10).max(10),
    )
}

#[derive(Clone)]
pub struct Overlay {
    window: ApplicationWindow,
    /// The line being sung, and the ones still to come under it.
    lines: gtk::Box,
    /// Fixed height, so the line can slide inside it without the surface
    /// growing and shrinking on every frame.
    stage: gtk::Box,
    current: Label,
    /// The same words in full colour, clipped to how far the song has gone.
    sung: Label,
    clip: gtk::Box,
    upcoming: Label,
    provider: CssProvider,
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
    /// The margins when the drag started, so the drag is relative to them.
    grabbed: Option<(i32, i32)>,
    positioning: bool,
}

impl Overlay {
    /// Builds the window and puts it on screen.
    pub fn build(app: &Application, settings: &Settings) -> Result<Self, Error> {
        let display = gtk::gdk::Display::default().ok_or(Error::NoDisplay)?;

        let provider = CssProvider::new();
        provider.load_from_string(&style(settings));
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let current = Label::builder()
            .justify(gtk::Justification::Center)
            .wrap(true)
            .opacity(0.0)
            .build();
        let sung = Label::builder()
            .justify(gtk::Justification::Center)
            .wrap(true)
            .xalign(0.0)
            .build();

        // The bright copy sits on top of the dim one and is cut off at the
        // point the song has reached.
        let clip = gtk::Box::builder()
            .halign(gtk::Align::Start)
            .overflow(gtk::Overflow::Hidden)
            .visible(false)
            .build();
        clip.append(&sung);

        let stacked = gtk::Overlay::builder().child(&current).build();
        stacked.add_overlay(&clip);

        let stage = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .overflow(gtk::Overflow::Hidden)
            .build();
        stage.append(&stacked);
        let upcoming = Label::builder()
            .justify(gtk::Justification::Center)
            .wrap(true)
            .visible(false)
            .build();
        upcoming.add_css_class("upcoming");

        let lines = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::End)
            .build();
        lines.add_css_class("lines");
        lines.append(&stage);
        lines.append(&upcoming);

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(900)
            .default_height(160)
            .child(&lines)
            .build();
        window.add_css_class(OVERLAY);

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
            lines,
            stage,
            current,
            sung,
            clip,
            upcoming,
            provider,
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

    /// How far through the line the song is, from 0 to 1.
    ///
    /// Does nothing unless karaoke is on, and nothing while the line is
    /// changing: a half-drawn fill under a sliding line reads as a glitch.
    pub fn show_progress(&self, progress: Option<f64>) {
        let karaoke = self.placement.borrow().settings.karaoke;
        let Some(progress) = progress.filter(|_| karaoke && !self.fade.borrow().running) else {
            self.clip.set_visible(false);
            self.current.remove_css_class("unsung");
            return;
        };

        let width = self.current.width();
        if width <= 0 {
            return;
        }
        self.current.add_css_class("unsung");
        self.sung.set_width_request(width);
        self.clip
            .set_width_request(((f64::from(width) * progress) as i32).max(1));
        self.clip.set_visible(true);
    }

    /// The lines still to come, under the one being sung.
    pub fn show_upcoming(&self, lines: &[String]) {
        if lines.is_empty() {
            self.upcoming.set_visible(false);
            return;
        }
        self.upcoming.set_text(&lines.join("\n"));
        self.upcoming.set_visible(true);
    }

    /// Takes the settings again, after the preferences window changed them.
    pub fn reload(&self, settings: &Settings) {
        self.provider.load_from_string(&style(settings));
        let movable = settings.movable;
        {
            let mut placement = self.placement.borrow_mut();
            placement.settings = settings.clone();
            placement.positioning = movable;
        }
        if movable {
            self.window.add_css_class(POSITIONING);
            self.current.set_opacity(1.0);
        } else {
            self.window.remove_css_class(POSITIONING);
        }
        self.place();
        self.set_click_through(!movable);
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
    /// an anchor and a margin, and moving it under the cursor would drag the
    /// cursor's own frame of reference along. So the mode grows the surface to
    /// cover the screen and moves the lines inside it, which stays still.
    pub fn toggle_positioning(&self) {
        let positioning = {
            let mut placement = self.placement.borrow_mut();
            placement.positioning = !placement.positioning;
            // The switch in the preferences window reads this, so the two
            // cannot be left disagreeing.
            placement.settings.movable = placement.positioning;
            placement.positioning
        };

        if positioning {
            self.window.add_css_class(POSITIONING);
            self.window.set_visible(true);
            self.current.set_opacity(1.0);
            if self.current.text().is_empty() {
                self.current.set_text("drag me");
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
        self.place();
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
                {
                    let mut placement = overlay.placement.borrow_mut();
                    let Some((left, bottom)) = placement.grabbed else {
                        return;
                    };
                    // Dragging down moves the lines down, which is a smaller
                    // distance from the bottom.
                    placement.settings.left_margin = Some((left + x as i32).max(0));
                    placement.settings.bottom_margin = (bottom - y as i32).max(0);
                }
                overlay.place();
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

    /// Puts the lines where the settings say.
    fn place(&self) {
        let placement = self.placement.borrow();
        let positioning = placement.positioning;
        let (left, bottom) = (
            placement.settings.left_margin,
            placement.settings.bottom_margin,
        );
        drop(placement);

        if !self.layer_shell {
            x11::place(&self.window, left, bottom);
            return;
        }

        self.choose_monitor();

        if positioning {
            // The surface covers the screen and stops moving; the lines move
            // inside it, so the pointer and the thing it drags stay in the
            // same frame of reference.
            for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                self.window.set_anchor(edge, true);
                self.window.set_margin(edge, 0);
            }
            self.lines.set_halign(gtk::Align::Start);
            self.lines
                .set_margin_start(left.unwrap_or_else(|| self.centred_left()));
            self.lines.set_margin_bottom(bottom);
            return;
        }

        self.lines.set_halign(gtk::Align::Center);
        self.lines.set_margin_start(0);
        self.lines.set_margin_bottom(0);
        self.window.set_anchor(Edge::Top, false);
        self.window.set_anchor(Edge::Right, false);
        self.window.set_anchor(Edge::Bottom, true);
        self.window.set_margin(Edge::Bottom, bottom);
        // Anchoring left is what makes the left margin mean anything; with no
        // anchor the compositor centres the surface.
        self.window.set_anchor(Edge::Left, left.is_some());
        self.window.set_margin(Edge::Left, left.unwrap_or(0));
    }

    /// Binds the surface to the screen the settings name.
    ///
    /// A layer surface belongs to one output. Moving between screens is not a
    /// drag but a different surface, so the screen is a setting.
    fn choose_monitor(&self) {
        let wanted = self.placement.borrow().settings.monitor.clone();
        let Some(wanted) = wanted else {
            return;
        };
        let Some(display) = gtk::gdk::Display::default() else {
            return;
        };

        let monitors = display.monitors();
        for index in 0..monitors.n_items() {
            let Some(monitor) = monitors.item(index).and_downcast::<gtk::gdk::Monitor>() else {
                continue;
            };
            if monitor.connector().is_some_and(|name| name == wanted) {
                self.window.set_monitor(Some(&monitor));
                return;
            }
        }
        tracing::warn!(%wanted, "no screen by that name; leaving it to the compositor");
    }

    /// Where the lines sit when centred, so a drag can start from there.
    fn centred_left(&self) -> i32 {
        let Some(surface) = self.window.surface() else {
            return 0;
        };
        let Some(monitor) = surface.display().monitor_at_surface(&surface) else {
            return 0;
        };
        (monitor.geometry().width() - self.lines.width().max(1)) / 2
    }

    /// Fades the current line out, swaps the text, fades the next one in.
    ///
    /// Driven by the frame clock rather than a timer, so it runs at whatever
    /// rate the screen actually refreshes, and stops as soon as it is done.
    fn animate(&self) {
        let started = Instant::now();
        let fade = Rc::clone(&self.fade);
        let current = self.current.clone();
        let sung = self.sung.clone();
        let clip = self.clip.clone();
        let stage = self.stage.clone();
        let swapped = std::cell::Cell::new(false);

        self.lines.add_tick_callback(move |_, _| {
            let elapsed = started.elapsed();

            // Out: the line fades where it stands.
            if elapsed < FADE {
                current.set_opacity(1.0 - elapsed.as_secs_f64() / FADE.as_secs_f64());
                return ControlFlow::Continue;
            }

            if !swapped.get() {
                swapped.set(true);
                clip.set_visible(false);
                let mut state = fade.borrow_mut();
                state.shown = state.wanted.clone();
                let text = state.shown.clone().unwrap_or_default();
                current.set_text(&text);
                sung.set_text(&text);
                // Fixing the height here is what lets the line slide inside a
                // box that does not resize, so the surface stays put.
                let (_, natural, _, _) = current.measure(gtk::Orientation::Vertical, -1);
                stage.set_size_request(-1, natural.max(1));
            }

            if fade.borrow().shown.is_none() {
                fade.borrow_mut().running = false;
                current.set_opacity(0.0);
                current.set_margin_top(0);
                return ControlFlow::Break;
            }

            // In: it comes up from below as it appears.
            let progress = ((elapsed - FADE).as_secs_f64() / FADE.as_secs_f64()).min(1.0);
            let eased = 1.0 - (1.0 - progress).powi(3);
            current.set_opacity(eased);
            current.set_margin_top((f64::from(RISE) * (1.0 - eased)) as i32);

            if progress >= 1.0 {
                current.set_margin_top(0);
                fade.borrow_mut().running = false;
                return ControlFlow::Break;
            }
            ControlFlow::Continue
        });
    }
}
