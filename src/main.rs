use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk::gio;
use gtk::glib;
use gtk4 as gtk;
use lyricslens::app::{self, Update};
use lyricslens::lyrics::Lyrics;
use lyricslens::media::{Event, Track};
use lyricslens::store::settings::Settings;
use lyricslens::sync::clock::Clock;
use lyricslens::ui::overlay::Overlay;
use lyricslens::ui::settings as preferences;

/// How often the overlay asks the clock which line is being sung.
///
/// The screen redraws at its own rate; this only decides how late a line can
/// be, and a tenth of a second is below what anyone sees.
const TICK: Duration = Duration::from_millis(100);

const ID: &str = "io.github.julio0cesar.lyricslens";

/// What the running overlay can be told to do from outside.
///
/// Wayland gives an ordinary client no way to grab a key combination, so the
/// hotkey belongs to the compositor. All this program offers is the command
/// for the compositor to run.
const COMMANDS: [(&str, &str); 3] = [
    ("--toggle", "toggle"),
    ("--position", "position"),
    ("--settings", "settings"),
];

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Some(code) = lyricslens::cli::handle() {
        return glib::ExitCode::from(code);
    }

    // Anything the running overlay can do, it should do itself rather than a
    // second copy doing it beside it.
    if let Some(action) = asked_command()
        && let Some(code) = send(action)
    {
        return code;
    }

    let application = adw::Application::builder().application_id(ID).build();

    if preferences::requested() {
        // Nothing is running, so the preferences window is the whole program.
        application.connect_activate(preferences::open);
        // GTK would try to make sense of our own flags otherwise.
        return application.run_with_args::<&str>(&[]);
    }

    // Launching again while a copy runs is a request to see it, not to start
    // another. Saying so is the difference between "it did nothing" and "it is
    // already there".
    if running_elsewhere() {
        println!("lyricslens is already running; bringing the overlay to the front");
        return send("present").unwrap_or(glib::ExitCode::SUCCESS);
    }

    let settings = Settings::load();
    let updates = app::start(settings.clone());

    // GTK calls this again every time the program is launched while one copy
    // is already running. Building a second overlay there would stack another
    // surface on the screen, so the running one is raised instead.
    let running: Rc<RefCell<Option<Rc<Overlay>>>> = Rc::new(RefCell::new(None));

    application.connect_activate(move |application| {
        if let Some(overlay) = running.borrow().as_ref() {
            overlay.present();
            return;
        }

        let overlay = match Overlay::build(application.upcast_ref(), &settings) {
            Ok(overlay) => overlay,
            Err(error) => {
                tracing::error!(%error, "could not open the overlay");
                return;
            }
        };

        let overlay = Rc::new(overlay);
        *running.borrow_mut() = Some(Rc::clone(&overlay));
        add_commands(application, &overlay);

        let state = Rc::new(RefCell::new(State::new(settings.clone())));

        let updates = updates.clone();
        let reader = Rc::clone(&state);
        glib::spawn_future_local(async move {
            while let Ok(update) = updates.recv().await {
                reader.borrow_mut().apply(update);
            }
        });

        glib::timeout_add_local(TICK, move || {
            overlay.show(state.borrow().line().as_deref());
            glib::ControlFlow::Continue
        });
    });

    application.run_with_args::<&str>(&[])
}

/// The action a command-line flag asks the running overlay for.
fn asked_command() -> Option<&'static str> {
    let arguments: Vec<String> = std::env::args().collect();
    COMMANDS
        .iter()
        .find(|(flag, _)| arguments.iter().any(|argument| argument == flag))
        .map(|(_, action)| *action)
}

/// Hands the action to the instance already running.
///
/// `None` means there is none, and the caller decides what to do about it: for
/// the preferences window that is to open one, for hiding the overlay there is
/// nothing to hide.
fn send(action: &str) -> Option<glib::ExitCode> {
    let remote = gio::Application::new(Some(ID), gio::ApplicationFlags::empty());
    if let Err(error) = remote.register(gio::Cancellable::NONE) {
        eprintln!("lyricslens: could not reach the session bus: {error}");
        return Some(glib::ExitCode::FAILURE);
    }
    if !remote.is_remote() {
        return None;
    }
    remote.activate_action(action, None);
    Some(glib::ExitCode::SUCCESS)
}

/// Whether another copy already holds the application's name on the bus.
fn running_elsewhere() -> bool {
    let probe = gio::Application::new(Some(ID), gio::ApplicationFlags::empty());
    probe.register(gio::Cancellable::NONE).is_ok() && probe.is_remote()
}

fn add_commands(application: &adw::Application, overlay: &Rc<Overlay>) {
    let present = gio::SimpleAction::new("present", None);
    present.connect_activate({
        let overlay = Rc::clone(overlay);
        move |_, _| overlay.present()
    });

    let settings = gio::SimpleAction::new("settings", None);
    settings.connect_activate({
        let application = application.clone();
        move |_, _| preferences::open(&application)
    });

    let toggle = gio::SimpleAction::new("toggle", None);
    toggle.connect_activate({
        let overlay = Rc::clone(overlay);
        move |_, _| overlay.toggle()
    });

    let position = gio::SimpleAction::new("position", None);
    position.connect_activate({
        let overlay = Rc::clone(overlay);
        move |_, _| overlay.toggle_positioning()
    });

    application.add_action(&present);
    application.add_action(&settings);
    application.add_action(&toggle);
    application.add_action(&position);
}

/// What the overlay is showing, and everything it takes to decide that.
struct State {
    settings: Settings,
    track: Track,
    lyrics: Option<Lyrics>,
    clock: Clock,
    stalled: bool,
    /// True between a track change and the answer about its lyrics, so the
    /// overlay can tell "still looking" from "there are none".
    searching: bool,
}

impl State {
    fn new(settings: Settings) -> Self {
        Self {
            settings,
            track: Track::default(),
            lyrics: None,
            clock: Clock::new(0),
            stalled: false,
            searching: false,
        }
    }

    fn apply(&mut self, update: Update) {
        match update {
            Update::Lyrics(lyrics) => {
                self.lyrics = *lyrics;
                self.searching = false;
            }
            Update::Player(name) => self.clock.set_offset_ms(self.settings.offset_ms(&name)),
            Update::Media(Event::TrackChanged(track)) => {
                self.track = track;
                self.lyrics = None;
                self.searching = true;
                self.stalled = false;
                self.clock.reset();
            }
            Update::Media(Event::Playback(state)) => {
                self.stalled = false;
                self.clock.playback(state, Instant::now());
            }
            Update::Media(Event::Position { reading, at }) => {
                self.clock.sample(reading, at);
            }
            Update::Media(Event::PositionStalled) => self.stalled = true,
        }
    }

    /// The words on screen right now.
    /// The words on screen right now.
    ///
    /// When there are no lyrics to follow, the overlay says why instead of
    /// staying blank — a blank overlay and a broken one look identical.
    fn line(&self) -> Option<String> {
        if self.track.is_empty() {
            return None;
        }

        let position = self.clock.position(Instant::now());
        if let (Some(lyrics), Some(position)) = (self.lyrics.as_ref(), position) {
            // None here is an instrumental gap, which is meant to be silent.
            return lyrics
                .line_at(position)
                .and_then(|line| line.sung())
                .map(str::to_owned);
        }

        let title = self.track.title.as_deref().unwrap_or("unknown track");
        let reason = if self.stalled {
            "this player does not report its position"
        } else if self.searching {
            "looking for the lyrics…"
        } else if self.lyrics.is_none() {
            "no synced lyrics for this one"
        } else {
            "waiting for the player"
        };
        Some(format!("{title}  ·  {reason}"))
    }
}
