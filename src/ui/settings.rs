//! The preferences window.
//!
//! The one place libadwaita earns its keep: rows that already look right and
//! already behave, instead of a hand-built grid. It never touches the overlay,
//! whose transparency libadwaita's own background would fight.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;

use crate::app::Request;
use crate::lyrics::lrclib::Candidate;
use crate::lyrics::normalize::from_track;
use crate::media::Track;
use crate::store::settings::Settings;

/// What the window needs to look lyrics up and hand a choice back.
///
/// Absent when the preferences were opened on their own, with no overlay
/// running: there is nothing playing to correct.
#[derive(Clone)]
pub struct Search {
    pub requests: async_channel::Sender<Request>,
    pub candidates: async_channel::Receiver<Vec<Candidate>>,
    pub playing: Rc<RefCell<Track>>,
    /// What became of the automatic search for the track playing now.
    pub report: Rc<RefCell<String>>,
}

/// Opens the preferences window, saving each change as it is made.
pub fn open(app: &adw::Application) {
    show(app, None);
}

/// Opens the preferences, with the means to correct the lyrics when there is a
/// track playing to correct them for.
pub fn show(app: &adw::Application, search: Option<Search>) {
    let settings = std::rc::Rc::new(std::cell::RefCell::new(Settings::load()));

    let window = adw::PreferencesWindow::builder()
        .application(app)
        .title("LyricsLens")
        .search_enabled(false)
        .default_width(520)
        .default_height(420)
        .build();

    let page = adw::PreferencesPage::new();

    let player = Section::new(
        "Player",
        "Which player to follow when more than one is open.",
    );
    let player_row = adw::EntryRow::builder()
        .title("Part of the bus name")
        .text(settings.borrow().player.clone().unwrap_or_default())
        .build();
    player_row.connect_changed({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            let text = row.text().trim().to_owned();
            settings.borrow_mut().player = (!text.is_empty()).then_some(text);
            save(&settings.borrow(), &app);
        }
    });
    player.add(&player_row);

    // A layer surface belongs to one screen, so this is a choice rather than a
    // drag across the edge.
    let screens = connectors();
    let mut names: Vec<&str> = vec!["Whichever the compositor picks"];
    names.extend(screens.iter().map(String::as_str));
    let screen = adw::ComboRow::builder()
        .title("Screen")
        .model(&gtk::StringList::new(&names))
        .selected(
            settings
                .borrow()
                .monitor
                .as_ref()
                .and_then(|wanted| screens.iter().position(|name| name == wanted))
                .map_or(0, |index| index as u32 + 1),
        )
        .build();
    screen.connect_selected_notify({
        let settings = settings.clone();
        let app = app.clone();
        let screens = screens.clone();
        move |row| {
            let selected = row.selected() as usize;
            settings.borrow_mut().monitor = selected
                .checked_sub(1)
                .and_then(|index| screens.get(index).cloned());
            save(&settings.borrow(), &app);
        }
    });

    let look = Section::new(
        "Appearance",
        "Every change here shows on the overlay straight away.",
    );

    let font = adw::SpinRow::with_range(12.0, 96.0, 1.0);
    font.set_title("Font size");
    font.set_value(f64::from(settings.borrow().font_size));
    font.connect_value_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            settings.borrow_mut().font_size = row.value().max(0.0) as u32;
            save(&settings.borrow(), &app);
        }
    });
    look.add(&font);

    let margin = adw::SpinRow::with_range(0.0, 800.0, 10.0);
    margin.set_title("Distance from the bottom");
    margin.set_subtitle("In pixels");
    margin.set_value(f64::from(settings.borrow().bottom_margin));
    margin.connect_value_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            settings.borrow_mut().bottom_margin = row.value() as i32;
            save(&settings.borrow(), &app);
        }
    });
    look.add(&margin);

    let colour = adw::EntryRow::builder()
        .title("Text colour")
        .text(settings.borrow().text_color.clone())
        .build();
    colour.connect_changed({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            let text = row.text().trim().to_owned();
            // A half-typed colour would blank the text until the last digit.
            if !looks_like_a_colour(&text) {
                return;
            }
            settings.borrow_mut().text_color = text;
            save(&settings.borrow(), &app);
        }
    });
    look.add(&colour);

    let shadow = adw::SwitchRow::builder()
        .title("Shadow under the text")
        .subtitle("What keeps it readable over a bright window")
        .active(settings.borrow().text_shadow)
        .build();
    shadow.connect_active_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            settings.borrow_mut().text_shadow = row.is_active();
            save(&settings.borrow(), &app);
        }
    });
    look.add(&shadow);

    let dim = adw::SpinRow::with_range(0.0, 100.0, 5.0);
    dim.set_title("Darkness behind the line");
    dim.set_subtitle("Per cent. Zero shows nothing behind the words");
    dim.set_value(settings.borrow().background_opacity * 100.0);
    dim.connect_value_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            settings.borrow_mut().background_opacity = row.value() / 100.0;
            save(&settings.borrow(), &app);
        }
    });
    look.add(&dim);

    let upcoming = adw::SpinRow::with_range(0.0, 3.0, 1.0);
    upcoming.set_title("Lines still to come");
    upcoming.set_subtitle(
        "Shown dimmed underneath. At least one is needed for the line to rise into place",
    );
    upcoming.set_value(f64::from(settings.borrow().upcoming_lines));
    upcoming.connect_value_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            settings.borrow_mut().upcoming_lines = row.value().max(0.0) as u8;
            save(&settings.borrow(), &app);
        }
    });
    look.add(&upcoming);

    let karaoke = adw::SwitchRow::builder()
        .title("Karaoke")
        .subtitle("Fills the line as the song moves through it")
        .active(settings.borrow().karaoke)
        .build();
    karaoke.connect_active_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            settings.borrow_mut().karaoke = row.is_active();
            save(&settings.borrow(), &app);
        }
    });
    look.add(&karaoke);

    let paused = adw::SwitchRow::builder()
        .title("Hide while paused")
        .subtitle("Lyrics on screen with nothing playing is the most confusing thing it can do")
        .active(settings.borrow().hide_when_paused)
        .build();
    paused.connect_active_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            settings.borrow_mut().hide_when_paused = row.is_active();
            save(&settings.borrow(), &app);
        }
    });
    look.add(&paused);

    let timing = Section::new(
        "Timing",
        "Positive holds the lyrics back, negative brings them forward.",
    );

    // The offset is kept per player, and which one is being followed is only
    // known while the overlay runs. Editing the one that is configured is what
    // this window can honestly offer.
    let followed = settings
        .borrow()
        .player
        .clone()
        .unwrap_or_else(|| "default".to_owned());
    let offset = adw::SpinRow::with_range(-5000.0, 5000.0, 50.0);
    offset.set_title("Offset in milliseconds");
    offset.set_subtitle(&followed);
    offset.set_value(settings.borrow().offset_ms(&followed) as f64);
    offset.connect_value_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            let value = row.value() as i64;
            settings.borrow_mut().set_offset_ms(&followed, value);
            save(&settings.borrow(), &app);
        }
    });
    timing.add(&offset);

    let place = Section::new(
        "Position",
        "A layer surface belongs to one screen and cannot be dragged to another.",
    );

    let movable = adw::SwitchRow::builder()
        .title("Let me move it")
        .subtitle("The overlay takes your clicks while this is on, so you can drag it")
        .active(settings.borrow().movable)
        .build();
    movable.connect_active_notify({
        let settings = settings.clone();
        let app = app.clone();
        move |row| {
            settings.borrow_mut().movable = row.is_active();
            save(&settings.borrow(), &app);
        }
    });
    place.add(&movable);
    place.add(&screen);

    // A program with no window of its own needs a way out that is not the
    // tray, for the desktops that have none.
    let close = Section::new("Closing", "");
    let quit = adw::ActionRow::builder()
        .title("Quit LyricsLens")
        .subtitle("Closes the overlay and leaves the status bar")
        .activatable(true)
        .build();
    quit.connect_activated({
        let app = app.clone();
        move |_| app.quit()
    });
    close.add(&quit);

    page.add(&player.group);
    if let Some(search) = search {
        page.add(&divider());
        page.add(&lyrics_group(&search).group);
    }
    for section in [&look, &place, &timing, &close] {
        page.add(&divider());
        page.add(&section.group);
    }
    window.add(&page);
    window.present();
}

/// Writes the settings down and tells the overlay to read them again.
fn save(settings: &Settings, app: &adw::Application) {
    if let Err(error) = settings.save() {
        tracing::warn!(%error, "could not save the settings");
        return;
    }
    app.activate_action("reload", None);
}

/// A heading with a chevron, and the rows it hides.
///
/// `adw::ExpanderRow` rather than a revealer of our own: a revealer draws the
/// rows on their way out past its edge, and they leave pieces of themselves
/// scattered between the sections.
#[derive(Clone)]
struct Section {
    group: adw::PreferencesGroup,
    expander: adw::ExpanderRow,
}

impl Section {
    fn new(title: &str, description: &str) -> Self {
        let expander = adw::ExpanderRow::builder()
            .title(title)
            .subtitle(description)
            .expanded(true)
            .build();

        let group = adw::PreferencesGroup::new();
        group.add(&expander);

        Self { group, expander }
    }

    fn add(&self, row: &impl IsA<gtk::Widget>) {
        self.expander.add_row(row);
    }
}

/// A line between two sections, which a page of groups has no other way to
/// draw.
fn divider() -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    let line = gtk::Separator::builder()
        .orientation(gtk::Orientation::Horizontal)
        .margin_top(2)
        .margin_bottom(2)
        .build();
    group.add(&line);
    group
}

/// The rescue for when the automatic match lands on the wrong recording.
///
/// Type a name, see everything the service has under it, pick one. The choice
/// is kept for the track playing now, so the same song comes back right.
fn lyrics_group(search: &Search) -> Section {
    let group = Section::new(
        "Lyrics for this track",
        "When the wrong words are on screen, find the right ones by hand.",
    );

    let query = from_track(&search.playing.borrow());
    let artist = adw::EntryRow::builder()
        .title("Artist")
        .text(query.artist.unwrap_or_default())
        .build();
    let title = adw::EntryRow::builder()
        .title("Title")
        .text(query.title)
        .build();

    let found_state = gtk::Label::builder()
        .halign(gtk::Align::Start)
        .margin_top(2)
        .margin_bottom(6)
        .build();
    found_state.add_css_class("dim-label");

    let results = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .visible(false)
        .build();
    results.add_css_class("boxed-list");

    let status = gtk::Label::builder()
        .label("")
        .halign(gtk::Align::Start)
        .visible(false)
        .build();
    status.add_css_class("dim-label");

    let button = gtk::Button::builder()
        .label("Search")
        .halign(gtk::Align::End)
        .margin_top(6)
        .build();
    button.add_css_class("suggested-action");
    button.connect_clicked({
        let search = search.clone();
        let artist = artist.clone();
        let title = title.clone();
        let results = results.clone();
        let status = status.clone();
        move |button| {
            let title_text = title.text().trim().to_owned();
            if title_text.is_empty() {
                status.set_label("A title is the least it needs.");
                status.set_visible(true);
                return;
            }

            button.set_sensitive(false);
            status.set_label("Searching…");
            status.set_visible(true);
            results.set_visible(false);

            let request = Request::Search {
                artist: artist.text().trim().to_owned(),
                title: title_text,
            };
            let search = search.clone();
            let results = results.clone();
            let status = status.clone();
            let button = button.clone();
            glib::spawn_future_local(async move {
                if search.requests.send(request).await.is_err() {
                    status.set_label("The overlay is not running.");
                    button.set_sensitive(true);
                    return;
                }
                let Ok(found) = search.candidates.recv().await else {
                    button.set_sensitive(true);
                    return;
                };
                fill(&results, &status, &search, found);
                button.set_sensitive(true);
            });
        }
    });

    group.add(&found_state);
    group.add(&artist);
    group.add(&title);
    group.add(&button);
    group.add(&status);
    group.add(&results);

    // The window outlives the song. When the track changes under it, the boxes
    // hold the name of something that is no longer playing and the list below
    // answers a question nobody is asking any more.
    let mut showing = search.playing.borrow().clone();
    glib::timeout_add_local(Duration::from_millis(500), {
        let search = search.clone();
        let artist = artist.clone();
        let title = title.clone();
        let results = results.clone();
        let status = status.clone();
        let found_state = found_state.clone();
        move || {
            // The window was closed; nothing left to keep in step.
            if artist.root().is_none() {
                return glib::ControlFlow::Break;
            }

            found_state.set_label(&search.report.borrow().clone());

            let playing = search.playing.borrow().clone();
            if playing == showing {
                return glib::ControlFlow::Continue;
            }
            showing = playing.clone();

            let query = from_track(&playing);
            artist.set_text(&query.artist.unwrap_or_default());
            title.set_text(&query.title);
            while let Some(row) = results.first_child() {
                results.remove(&row);
            }
            results.set_visible(false);
            status.set_visible(false);
            glib::ControlFlow::Continue
        }
    });

    group
}

/// Draws the answer, one row per recording.
fn fill(results: &gtk::ListBox, status: &gtk::Label, search: &Search, found: Vec<Candidate>) {
    while let Some(row) = results.first_child() {
        results.remove(&row);
    }

    if found.is_empty() {
        status.set_label("Nothing found under that name.");
        status.set_visible(true);
        results.set_visible(false);
        return;
    }

    status.set_label(&format!("{} with synced lyrics.", found.len()));
    status.set_visible(true);

    for candidate in found {
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&candidate.title))
            .subtitle(glib::markup_escape_text(&format!(
                "{}{} · {}",
                candidate.artist,
                if candidate.album.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", candidate.album)
                },
                length(candidate.length),
            )))
            .activatable(true)
            .build();
        row.connect_activated({
            let search = search.clone();
            let candidate = candidate.clone();
            move |row| {
                let search = search.clone();
                let candidate = candidate.clone();
                row.set_subtitle("Now showing these");
                glib::spawn_future_local(async move {
                    let _ = search
                        .requests
                        .send(Request::Choose(Box::new(candidate)))
                        .await;
                });
            }
        });
        results.append(&row);
    }
    results.set_visible(true);
}

fn length(length: Option<Duration>) -> String {
    match length {
        Some(length) => {
            let seconds = length.as_secs();
            format!("{}:{:02}", seconds / 60, seconds % 60)
        }
        None => "--:--".to_owned(),
    }
}

/// The names of the screens attached right now.
fn connectors() -> Vec<String> {
    let Some(display) = gtk::gdk::Display::default() else {
        return Vec::new();
    };
    let monitors = display.monitors();
    (0..monitors.n_items())
        .filter_map(|index| monitors.item(index).and_downcast::<gtk::gdk::Monitor>())
        .filter_map(|monitor| monitor.connector().map(|name| name.to_string()))
        .collect()
}

/// Enough of a check to keep a half-typed colour from blanking the text.
fn looks_like_a_colour(value: &str) -> bool {
    let hex = value.strip_prefix('#');
    match hex {
        Some(digits) => {
            matches!(digits.len(), 3 | 4 | 6 | 8) && digits.chars().all(|c| c.is_ascii_hexdigit())
        }
        // Named colours are fine too, and GTK knows more of them than we do.
        None => !value.is_empty() && value.chars().all(|c| c.is_ascii_alphabetic()),
    }
}

/// True when the program was asked for the preferences window rather than the
/// overlay.
pub fn requested() -> bool {
    std::env::args().any(|argument| argument == "--settings")
}
