use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{Application, glib};
use gtk4 as gtk;
use lyricslens::app::{self, Update};
use lyricslens::lyrics::Lyrics;
use lyricslens::media::{Event, Track};
use lyricslens::store::settings::Settings;
use lyricslens::sync::clock::Clock;
use lyricslens::ui::overlay::Overlay;

/// How often the overlay asks the clock which line is being sung.
///
/// The screen redraws at its own rate; this only decides how late a line can
/// be, and a tenth of a second is below what anyone sees.
const TICK: Duration = Duration::from_millis(100);

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let settings = Settings::load();
    let updates = app::start(settings.clone());

    let application = Application::builder()
        .application_id("io.github.julio0cesar.lyricslens")
        .build();

    application.connect_activate(move |application| {
        let overlay = match Overlay::build(application, &settings) {
            Ok(overlay) => overlay,
            Err(error) => {
                tracing::error!(%error, "could not open the overlay");
                return;
            }
        };

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

    application.run()
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
