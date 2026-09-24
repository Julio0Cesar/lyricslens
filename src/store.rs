//! What survives a restart: settings and the lyrics already fetched.

pub mod cache;
pub mod settings;

use std::path::PathBuf;

/// `$XDG_CONFIG_HOME/lyricslens`, or `~/.config/lyricslens`.
pub fn config_dir() -> Option<PathBuf> {
    base("XDG_CONFIG_HOME", ".config")
}

/// `$XDG_CACHE_HOME/lyricslens`, or `~/.cache/lyricslens`.
pub fn cache_dir() -> Option<PathBuf> {
    base("XDG_CACHE_HOME", ".cache")
}

fn base(variable: &str, fallback: &str) -> Option<PathBuf> {
    let root = match std::env::var_os(variable) {
        // The specification says a relative value is to be ignored.
        Some(value) if PathBuf::from(&value).is_absolute() => PathBuf::from(value),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(fallback),
    };
    Some(root.join("lyricslens"))
}
