//! Starting with the session.
//!
//! The mechanism is the freedesktop one: a `.desktop` file in
//! `~/.config/autostart/`, which every relevant desktop reads. No daemon, no
//! user service, nothing left behind to uninstall.
//!
//! The state *is* the existence of that file, and it is read from there rather
//! than kept in the settings. So the switch can never show on for something
//! that is off: delete the file by hand and the program agrees the next time
//! it is asked.

use std::path::PathBuf;

const FILE: &str = "lyricslens.desktop";

fn path() -> Option<PathBuf> {
    // `config_dir` already points inside the program's own folder.
    let config = super::config_dir()?.parent()?.to_path_buf();
    Some(config.join("autostart").join(FILE))
}

/// Whether the program is set to start with the session.
pub fn enabled() -> bool {
    path().is_some_and(|path| path.exists())
}

/// Turns it on or off, and says whether that worked.
pub fn set(enabled: bool) -> std::io::Result<()> {
    let Some(path) = path() else {
        return Ok(());
    };

    if !enabled {
        return match std::fs::remove_file(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        };
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // The binary by its full path: `~/.local/bin` is not always on the PATH a
    // session hands to the programs it starts.
    let program = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "lyricslens".to_owned());

    std::fs::write(
        &path,
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=LyricsLens\n\
             Comment=Synced lyrics on top of any application\n\
             Exec={program}\n\
             Icon=lyricslens\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_the_state() {
        let home = std::env::temp_dir().join("lyricslens-autostart-test");
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &home) };

        assert!(!enabled(), "nothing has been written yet");
        set(true).expect("writing the entry");
        assert!(enabled());
        set(false).expect("removing the entry");
        assert!(!enabled());
        // Turning it off twice is not an error.
        set(false).expect("removing it again");

        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        std::fs::remove_dir_all(&home).ok();
    }
}
