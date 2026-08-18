use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no display available")]
    NoDisplay,
}
