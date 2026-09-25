//! Where the program writes down what happened.
//!
//! Started from the application menu there is no terminal to print to, so a
//! failure would leave nothing behind and a bug report nothing to quote. The
//! log goes to a file instead, and `--foreground` keeps it on the terminal for
//! whoever is watching.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;

const FILE: &str = "lyricslens.log";

/// Past this the log is rolled over. One previous file is kept, so a problem
/// that happened yesterday is still there and a week of them is not.
const LIMIT: u64 = 1024 * 1024;

/// `$XDG_STATE_HOME/lyricslens`, or `~/.local/state/lyricslens`.
pub fn dir() -> Option<PathBuf> {
    let root = match std::env::var_os("XDG_STATE_HOME") {
        Some(value) if PathBuf::from(&value).is_absolute() => PathBuf::from(value),
        _ => PathBuf::from(std::env::var_os("HOME")?)
            .join(".local")
            .join("state"),
    };
    Some(root.join("lyricslens"))
}

pub fn path() -> Option<PathBuf> {
    Some(dir()?.join(FILE))
}

/// Sends the log to the terminal, or to the file when there is no terminal to
/// send it to.
pub fn start(foreground: bool) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("lyricslens=info"));

    if foreground {
        tracing_subscriber::fmt().with_env_filter(filter).init();
        return;
    }

    let Some(file) = Rolling::open() else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
        return;
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .init();
}

/// A file that starts again once it has grown past [`LIMIT`].
struct Rolling {
    file: File,
    written: u64,
}

impl Rolling {
    fn open() -> Option<Self> {
        let path = path()?;
        std::fs::create_dir_all(path.parent()?).ok()?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()?;
        let written = file.metadata().ok()?.len();
        Some(Self { file, written })
    }

    fn roll(&mut self) -> io::Result<()> {
        let Some(path) = path() else {
            return Ok(());
        };
        std::fs::rename(&path, path.with_extension("log.1"))?;
        self.file = OpenOptions::new().create(true).append(true).open(&path)?;
        self.written = 0;
        Ok(())
    }
}

impl Write for Rolling {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.written >= LIMIT {
            // A failed roll is not worth losing the line over: keep writing to
            // the file that is already open.
            let _ = self.roll();
        }
        let written = self.file.write(buffer)?;
        self.written += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
