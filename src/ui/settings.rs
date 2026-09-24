//! The preferences window.
//!
//! The one place libadwaita earns its keep: rows that already look right and
//! already behave, instead of a hand-built grid. It never touches the overlay,
//! whose transparency libadwaita's own background would fight.

use adw::prelude::*;

use crate::store::settings::Settings;

/// What a change to the settings is worth saying out loud.
const RESTART: &str = "Changes apply the next time LyricsLens starts.";

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
        move |row| {
            let text = row.text().trim().to_owned();
            settings.borrow_mut().player = (!text.is_empty()).then_some(text);
            save(&settings.borrow());
        }
    });
    player.add(&player_row);

    let look = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description(RESTART)
        .build();

    let font = adw::SpinRow::with_range(12.0, 96.0, 1.0);
    font.set_title("Font size");
    font.set_value(f64::from(settings.borrow().font_size));
    font.connect_value_notify({
        let settings = settings.clone();
        move |row| {
            settings.borrow_mut().font_size = row.value().max(0.0) as u32;
            save(&settings.borrow());
        }
    });
    look.add(&font);

    let margin = adw::SpinRow::with_range(0.0, 800.0, 10.0);
    margin.set_title("Distance from the bottom");
    margin.set_subtitle("In pixels");
    margin.set_value(f64::from(settings.borrow().bottom_margin));
    margin.connect_value_notify({
        let settings = settings.clone();
        move |row| {
            settings.borrow_mut().bottom_margin = row.value() as i32;
            save(&settings.borrow());
        }
    });
    look.add(&margin);

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
        move |row| {
            let value = row.value() as i64;
            settings.borrow_mut().set_offset_ms(&followed, value);
            save(&settings.borrow());
        }
    });
    timing.add(&offset);

    page.add(&player);
    page.add(&look);
    page.add(&timing);
    window.add(&page);
    window.present();
}

fn save(settings: &Settings) {
    if let Err(error) = settings.save() {
        tracing::warn!(%error, "could not save the settings");
    }
}

/// True when the program was asked for the preferences window rather than the
/// overlay.
pub fn requested() -> bool {
    std::env::args().any(|argument| argument == "--settings")
}
