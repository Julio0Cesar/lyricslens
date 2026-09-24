//! The preferences window.
//!
//! The one place libadwaita earns its keep: rows that already look right and
//! already behave, instead of a hand-built grid. It never touches the overlay,
//! whose transparency libadwaita's own background would fight.

use adw::prelude::*;
use gtk4 as gtk;

use crate::store::settings::Settings;

/// Opens the preferences window, saving each change as it is made.
pub fn open(app: &adw::Application) {
    let settings = std::rc::Rc::new(std::cell::RefCell::new(Settings::load()));

    let window = adw::PreferencesWindow::builder()
        .application(app)
        .title("LyricsLens")
        .search_enabled(false)
        .default_width(520)
        .default_height(420)
        .build();

    let page = adw::PreferencesPage::new();

    let player = adw::PreferencesGroup::builder()
        .title("Player")
        .description("Which player to follow when more than one is open.")
        .build();
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

    let look = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description("Every change here shows on the overlay straight away.")
        .build();

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

    let timing = adw::PreferencesGroup::builder()
        .title("Timing")
        .description("Positive holds the lyrics back, negative brings them forward.")
        .build();

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

    let place = adw::PreferencesGroup::builder()
        .title("Position")
        .description("A layer surface belongs to one screen and cannot be dragged to another.")
        .build();

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
    let close = adw::PreferencesGroup::new();
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

    page.add(&player);
    page.add(&look);
    page.add(&place);
    page.add(&timing);
    page.add(&close);
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
