//! Persistência local.

pub mod autostart;
pub mod cache;
pub mod settings;

pub use cache::Cache;
pub use settings::{Settings, SettingsStore};
