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
    window.connect_map(move |window| {
        let Some(id) = window_id(window) else {
            tracing::warn!("not an X11 surface; the overlay will behave as a normal window");
            return;
        };
        if let Err(error) = raise(id) {
            tracing::warn!(%error, "could not place the overlay above the other windows");
        }
        place(window, None, bottom_margin);
    });
}

/// Moves the window, the only way an X11 client can: by asking for it.
pub fn place(window: &gtk::ApplicationWindow, left: Option<i32>, bottom_margin: i32) {
    let Some(id) = window_id(window) else {
        return;
    };
    let size = (window.width(), window.height());
    if let Err(error) = move_to(id, size, left, bottom_margin) {
        tracing::debug!(%error, "could not move the overlay");
    }
}

fn window_id(window: &gtk::ApplicationWindow) -> Option<Window> {
    let surface = window.surface()?;
    let surface = surface.downcast::<X11Surface>().ok()?;
    Window::try_from(surface.xid()).ok()
}

type Failure = Box<dyn std::error::Error>;

fn raise(id: Window) -> Result<(), Failure> {
    let (connection, screen) = x11rb::connect(None)?;
    let root = connection.setup().roots[screen].root;

    let atom = |name: &str| -> Result<u32, Failure> {
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
    connection.flush()?;
    Ok(())
}

fn move_to(
    id: Window,
    size: (i32, i32),
    left: Option<i32>,
    bottom_margin: i32,
) -> Result<(), Failure> {
    let (connection, screen) = x11rb::connect(None)?;
    let screen = &connection.setup().roots[screen];

    let x = left.unwrap_or_else(|| (i32::from(screen.width_in_pixels) - size.0) / 2);
    let y = i32::from(screen.height_in_pixels) - size.1 - bottom_margin;
    connection.configure_window(id, &ConfigureWindowAux::new().x(x).y(y))?;
    connection.flush()?;
    Ok(())
}
