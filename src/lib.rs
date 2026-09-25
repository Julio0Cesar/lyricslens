//! Synchronized lyrics overlay for the desktop.
//!
//! The binary is a thin shell over these modules, which are also what the
//! integration tests drive.

pub mod app;
pub mod cli;
pub mod desktop;
pub mod error;
pub mod log;
pub mod lyrics;
pub mod media;
pub mod store;
pub mod sync;
pub mod ui;
