//! The icon in the status bar.
//!
//! A program that sits in the background with no window of its own is
//! invisible without one: there is nothing to click, and no way to close it.
//!
//! GTK4 has no tray of its own. This speaks StatusNotifierItem, the protocol
//! every modern bar implements, over D-Bus.

use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};

/// What the menu asks the interface to do. The tray lives on its own thread,
/// so it can only send; the GTK side does the work.
#[derive(Debug, Clone, Copy)]
pub enum Command {
    Toggle,
    Position,
    Settings,
    Quit,
}

struct Tray {
    commands: async_channel::Sender<Command>,
}

impl Tray {
    fn send(&self, command: Command) {
        // Full means the interface is busy and will catch up; closed means it
        // is already gone. Neither is worth a crash from a menu click.
        if let Err(error) = self.commands.try_send(command) {
            tracing::debug!(%error, ?command, "the interface did not take the command");
        }
    }
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "lyricslens".to_owned()
    }

    fn title(&self) -> String {
        "LyricsLens".to_owned()
    }

    /// The name the installer writes into the icon theme.
    fn icon_name(&self) -> String {
        "lyricslens".to_owned()
    }

    /// A left click is the thing people try first.
    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(Command::Toggle);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Show / hide".into(),
                activate: Box::new(|tray: &mut Self| tray.send(Command::Toggle)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Move the overlay".into(),
                activate: Box::new(|tray: &mut Self| tray.send(Command::Position)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Preferences…".into(),
                activate: Box::new(|tray: &mut Self| tray.send(Command::Settings)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|tray: &mut Self| tray.send(Command::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Puts the icon in the bar and hands back what the menu asks for.
///
/// A desktop with no status bar simply has nowhere to put it; the overlay runs
/// exactly the same, so this never fails the program.
pub fn start() -> async_channel::Receiver<Command> {
    let (commands, receiver) = async_channel::bounded(8);

    std::thread::Builder::new()
        .name("tray".to_owned())
        .spawn(move || match (Tray { commands }).spawn() {
            // The handle has to outlive the icon, and the icon lives as long
            // as the program does.
            Ok(handle) => std::mem::forget(handle),
            Err(error) => tracing::info!(%error, "no status bar to put an icon in"),
        })
        .expect("spawning a thread");

    receiver
}
