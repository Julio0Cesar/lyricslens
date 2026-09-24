//! Keeping the overlay above everything on X11.
//!
//! Without wlr-layer-shell there is no protocol that says "stay on top", so
//! the window asks the window manager for `_NET_WM_STATE_ABOVE` and places
//! itself. GTK4 dropped the calls that used to do this, so it goes through a
//! plain X11 connection of our own.
//!
//! This does not survive a window in true fullscreen. That is a limitation of
//! the fallback, not a bug to be fixed here.

use gdk4_x11::X11Surface;
use gtk::prelude::*;
use gtk4 as gtk;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    ClientMessageEvent, ConfigureWindowAux, ConnectionExt, EventMask, Window,
};

/// `_NET_WM_STATE` asks for a state to be added.
const ADD: u32 = 1;

/// The message says it comes from a normal application, not a pager.
const FROM_APPLICATION: u32 = 1;

/// Asks the window manager to keep this window on top, and puts it near the
/// bottom of the screen.
///
/// Every failure here is survivable: the overlay still shows, just as an
/// ordinary window, so nothing is propagated.
pub fn keep_above(window: &gtk::ApplicationWindow, bottom_margin: i32) {
    let margin = bottom_margin;
    window.connect_map(move |window| {
        let Some(id) = window_id(window) else {
            tracing::warn!("not an X11 surface; the overlay will behave as a normal window");
            return;
        };
        let size = (window.width(), window.height());
        if let Err(error) = place(id, size, margin) {
            tracing::warn!(%error, "could not place the overlay above the other windows");
        }
    });
}

fn window_id(window: &gtk::ApplicationWindow) -> Option<Window> {
    let surface = window.surface()?;
    let surface = surface.downcast::<X11Surface>().ok()?;
    Window::try_from(surface.xid()).ok()
}

fn place(
    id: Window,
    size: (i32, i32),
    bottom_margin: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (connection, screen) = x11rb::connect(None)?;
    let screen = &connection.setup().roots[screen];
    let root = screen.root;

    let atom = |name: &str| -> Result<u32, Box<dyn std::error::Error>> {
        Ok(connection
            .intern_atom(false, name.as_bytes())?
            .reply()?
            .atom)
    };
    let state = atom("_NET_WM_STATE")?;

    for wanted in ["_NET_WM_STATE_ABOVE", "_NET_WM_STATE_SKIP_TASKBAR"] {
        let event =
            ClientMessageEvent::new(32, id, state, [ADD, atom(wanted)?, 0, FROM_APPLICATION, 0]);
        connection.send_event(
            false,
            root,
            // The window manager listens for these on the root window.
            EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT,
            event,
        )?;
    }

    let x = (i32::from(screen.width_in_pixels) - size.0) / 2;
    let y = i32::from(screen.height_in_pixels) - size.1 - bottom_margin;
    connection.configure_window(id, &ConfigureWindowAux::new().x(x).y(y))?;
    connection.flush()?;
    Ok(())
}
