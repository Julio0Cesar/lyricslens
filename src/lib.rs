//! Synchronized lyrics overlay for the desktop.
//!
//! The binary is a thin shell over these modules, which are also what the
//! integration tests drive.

pub mod app;
pub mod error;
pub mod lyrics;
pub mod media;
pub mod sync;
pub mod ui;
