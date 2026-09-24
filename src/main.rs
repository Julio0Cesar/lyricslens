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
const COMMANDS: [(&str, &str); 2] = [("--toggle", "toggle"), ("--position", "position")];

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Some(action) = asked_command() {
        return send(action);
    }

    let application = adw::Application::builder().application_id(ID).build();

    if preferences::requested() {
        application.connect_activate(preferences::open);
        // GTK would try to make sense of our own flags otherwise.
        return application.run_with_args::<&str>(&[]);
    }

    let settings = Settings::load();
    let updates = app::start(settings.clone());

    application.connect_activate(move |application| {
        let overlay = match Overlay::build(application.upcast_ref(), &settings) {
            Ok(overlay) => overlay,
            Err(error) => {
                tracing::error!(%error, "could not open the overlay");
                return;
            }
        };

        let overlay = Rc::new(overlay);
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
            overlay.show(state.borrow().line());
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

/// Hands the action to the instance already running, and says so when there is
/// none.
fn send(action: &str) -> glib::ExitCode {
    let remote = gio::Application::new(Some(ID), gio::ApplicationFlags::empty());
    if let Err(error) = remote.register(gio::Cancellable::NONE) {
        eprintln!("could not reach the session bus: {error}");
        return glib::ExitCode::FAILURE;
    }
    if !remote.is_remote() {
        eprintln!("lyricslens is not running");
        return glib::ExitCode::FAILURE;
    }
    remote.activate_action(action, None);
    glib::ExitCode::SUCCESS
}

fn add_commands(application: &adw::Application, overlay: &Rc<Overlay>) {
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
}

impl State {
    fn new(settings: Settings) -> Self {
        Self {
            settings,
            track: Track::default(),
            lyrics: None,
            clock: Clock::new(0),
            stalled: false,
        }
    }

    fn apply(&mut self, update: Update) {
        match update {
            Update::Lyrics(lyrics) => self.lyrics = *lyrics,
            Update::Player(name) => self.clock.set_offset_ms(self.settings.offset_ms(&name)),
            Update::Media(Event::TrackChanged(track)) => {
                self.track = track;
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
    fn line(&self) -> Option<&str> {
        let lyrics = self.lyrics.as_ref()?;
        let position = self.clock.position(Instant::now())?;
        lyrics.line_at(position)?.sung()
    }
}
