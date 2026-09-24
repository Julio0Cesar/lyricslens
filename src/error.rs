use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no display available")]
    NoDisplay,

    #[error("session bus: {0}")]
    Bus(#[from] zbus::Error),

    #[error("D-Bus call failed: {0}")]
    Call(#[from] zbus::fdo::Error),

    #[error("malformed D-Bus name: {0}")]
    Name(#[from] zbus::names::Error),

    #[error("lyrics service: {0}")]
    Http(#[from] reqwest::Error),
}
