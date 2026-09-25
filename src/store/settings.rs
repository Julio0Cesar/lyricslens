//! Settings, kept as TOML the user can read and edit.
//!
//! A missing file means the defaults. A corrupted one is reported and then
//! ignored, because losing the overlay over a stray bracket would be worse
//! than losing the settings.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE: &str = "settings.toml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Part of a player's bus name, when more than one is running.
    pub player: Option<String>,
    /// The key combination that shows and hides the overlay, written the way
    /// Hyprland writes it: `SUPER SHIFT, L`. Empty asks for nothing.
    pub hotkey_toggle: String,
    /// The same, for the mode where the overlay can be dragged.
    pub hotkey_position: String,
    /// Which screen the overlay lives on, by connector name — `HDMI-A-1` and
    /// the like. A layer surface belongs to one screen and cannot be dragged
    /// to another, so it is chosen rather than moved. Absent means whichever
    /// the compositor picks.
    pub monitor: Option<String>,
    /// How far above the bottom of the screen the line sits, in pixels.
    pub bottom_margin: i32,
    /// How far from the left edge, in pixels. Absent means centred, which is
    /// what it is until the overlay is dragged somewhere else.
    pub left_margin: Option<i32>,
    pub font_size: u32,
    /// The font by name, as the desktop knows it. Empty follows the desktop.
    pub font_family: String,
    /// From 100 to 900, the way a font names its weights.
    pub font_weight: u32,
    /// `center`, `start` or `end`. Anything else is read as `center`.
    pub align: String,
    /// How wide the overlay may get before a line wraps, in pixels.
    pub max_width: u32,
    /// How round the corners of the strip are, in pixels.
    pub corner_radius: u32,
    /// The colour of the line being sung.
    pub text_color: String,
    /// A shadow under the text, which is what keeps it readable over a bright
    /// window without a background of its own.
    pub text_shadow: bool,
    /// How dark the strip behind the line is, from 0 for nothing to 1 for
    /// solid black. Anything above zero also rounds its corners.
    pub background_opacity: f64,
    /// Shows the line just sung above the current one, dimmed.
    pub previous_line: bool,
    /// How many lines still to come are shown under the current one, dimmed.
    /// Zero shows only what is being sung now.
    pub upcoming_lines: u8,
    /// Fills the line word by word as the song moves through it.
    pub karaoke: bool,
    /// Clears the overlay while the player is paused. Lyrics on screen with
    /// nothing coming out of the speakers is the most confusing thing this
    /// program can do.
    pub hide_when_paused: bool,
    /// Lets the pointer reach the overlay, so it can be dragged. Off means
    /// clicks go straight through to whatever is underneath.
    pub movable: bool,
    /// Manual correction per player, in milliseconds. Positive means the
    /// lyrics run early and have to wait.
    pub offsets: BTreeMap<String, i64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            player: None,
            monitor: None,
            hotkey_toggle: String::new(),
            hotkey_position: String::new(),
            bottom_margin: 100,
            left_margin: None,
            font_size: 30,
            font_family: String::new(),
            font_weight: 600,
            align: "center".to_owned(),
            max_width: 900,
            corner_radius: 14,
            text_color: "#ffffff".to_owned(),
            text_shadow: true,
            background_opacity: 0.0,
            previous_line: false,
            upcoming_lines: 0,
            karaoke: false,
            hide_when_paused: true,
            movable: false,
            offsets: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// The alignment as GTK spells it, with anything unrecognised centred.
    pub fn alignment(&self) -> &'static str {
        match self.align.trim().to_ascii_lowercase().as_str() {
            "start" | "left" => "start",
            "end" | "right" => "end",
            _ => "center",
        }
    }

    /// Reads the settings, falling back to the defaults for anything missing.
    pub fn load() -> Self {
        let Some(path) = path() else {
            return Self::default();
        };
        Self::read(&path)
    }

    fn read(path: &Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "could not read the settings");
                return Self::default();
            }
        };

        match toml::from_str(&text) {
            Ok(settings) => settings,
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "the settings file is not valid; using the defaults");
                Self::default()
            }
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        std::fs::write(path, text)
    }

    /// The correction for one player, by its bus name.
    pub fn offset_ms(&self, player: &str) -> i64 {
        self.offsets.get(player).copied().unwrap_or(0)
    }

    pub fn set_offset_ms(&mut self, player: &str, offset_ms: i64) {
        if offset_ms == 0 {
            self.offsets.remove(player);
        } else {
            self.offsets.insert(player.to_owned(), offset_ms);
        }
    }
}

fn path() -> Option<PathBuf> {
    Some(super::config_dir()?.join(FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_means_the_defaults() {
        let settings = Settings::read(Path::new("/nonexistent/lyricslens/settings.toml"));
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn a_corrupted_file_does_not_panic() {
        let path = std::env::temp_dir().join("lyricslens-broken.toml");
        std::fs::write(&path, "player = [this is not toml").expect("writing the file");

        assert_eq!(Settings::read(&path), Settings::default());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_field_left_out_keeps_its_default() {
        let settings: Settings = toml::from_str("font_size = 44").expect("valid toml");
        assert_eq!(settings.font_size, 44);
        assert_eq!(settings.bottom_margin, Settings::default().bottom_margin);
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let mut settings = Settings {
            player: Some("spotify".to_owned()),
            ..Settings::default()
        };
        settings.set_offset_ms("org.mpris.MediaPlayer2.spotify", -250);

        let text = toml::to_string_pretty(&settings).expect("serialisable");
        assert_eq!(
            toml::from_str::<Settings>(&text).expect("valid toml"),
            settings
        );
    }

    #[test]
    fn a_zero_offset_is_not_stored() {
        let mut settings = Settings::default();
        settings.set_offset_ms("a", 300);
        settings.set_offset_ms("a", 0);
        assert!(settings.offsets.is_empty());
        assert_eq!(settings.offset_ms("a"), 0);
    }

    #[test]
    fn a_relative_xdg_path_is_ignored() {
        // The specification says so, and honouring it would put the settings
        // somewhere relative to wherever the app happened to start.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "relative/path") };
        let dir = super::super::config_dir().expect("a directory");
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        assert!(dir.is_absolute(), "{dir:?}");
    }
}
