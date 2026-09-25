//! Asking the compositor for a key combination.
//!
//! Wayland gives an ordinary client no way to claim one: the compositor owns
//! every key, and a program that wants one has to ask. Hyprland and Sway both
//! take the request over their own control socket.
//!
//! Nothing is written to the user's configuration. The compositor forgets the
//! binding when it restarts, and this asks again every time the program
//! starts — so there is nothing to clean up, and nothing left behind pointing
//! at a program that has been removed.
//!
//! Hyprland stopped taking `hyprctl keyword` in 0.56, where the configuration
//! moved to Lua; asking fails there, and the preferences window offers the
//! line to paste instead.

use std::process::Command;

/// The compositors that can be asked, and everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compositor {
    Hyprland,
    Sway,
    /// Somewhere else. The key stays the user's to configure.
    Unknown,
}

pub fn compositor() -> Compositor {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        return Compositor::Hyprland;
    }
    if std::env::var_os("SWAYSOCK").is_some() {
        return Compositor::Sway;
    }
    Compositor::Unknown
}

/// Asks for `combination` to run this program with `flag`.
///
/// The combination is written the way Hyprland writes it — `SUPER SHIFT, L` —
/// and translated for Sway. An empty one asks for nothing.
pub fn bind(combination: &str, flag: &str) -> Result<(), String> {
    let combination = combination.trim();
    if combination.is_empty() {
        return Ok(());
    }

    let program = std::env::current_exe()
        .map_err(|error| format!("could not find my own binary: {error}"))?
        .display()
        .to_string();

    let (command, arguments) = match compositor() {
        Compositor::Hyprland => {
            let (modifiers, key) = split(combination)?;
            (
                "hyprctl",
                vec![
                    "keyword".to_owned(),
                    "bind".to_owned(),
                    format!("{modifiers},{key},exec,{program} {flag}"),
                ],
            )
        }
        Compositor::Sway => {
            let (modifiers, key) = split(combination)?;
            (
                "swaymsg",
                vec![
                    "bindsym".to_owned(),
                    sway_keys(&modifiers, &key),
                    "exec".to_owned(),
                    format!("{program} {flag}"),
                ],
            )
        }
        Compositor::Unknown => {
            return Err("this desktop has no way to be asked; bind the key yourself".to_owned());
        }
    };

    let output = Command::new(command)
        .args(&arguments)
        .output()
        .map_err(|error| format!("{command} could not be run: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "{command} refused: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// The line to put in a configuration file, for a desktop that will not be
/// asked.
pub fn snippet(combination: &str, flag: &str) -> String {
    let program = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "lyricslens".to_owned());
    let combination = if combination.trim().is_empty() {
        "SUPER, L"
    } else {
        combination.trim()
    };

    match compositor() {
        Compositor::Sway => {
            let (modifiers, key) = split(combination).unwrap_or_default();
            format!(
                "bindsym {} exec {program} {flag}",
                sway_keys(&modifiers, &key)
            )
        }
        _ => {
            let (modifiers, key) = split(combination).unwrap_or_default();
            format!("bind = {modifiers}, {key}, exec, {program} {flag}")
        }
    }
}

/// `SUPER SHIFT, L` into its two halves.
fn split(combination: &str) -> Result<(String, String), String> {
    match combination.rsplit_once(',') {
        Some((modifiers, key)) => Ok((
            modifiers.split_whitespace().collect::<Vec<_>>().join(" "),
            key.trim().to_owned(),
        )),
        // A key on its own is allowed, with no modifier.
        None => Ok((String::new(), combination.to_owned())),
    }
}

/// Sway spells the modifiers differently and joins them with a plus.
fn sway_keys(modifiers: &str, key: &str) -> String {
    let mut parts: Vec<String> = modifiers
        .split_whitespace()
        .map(|modifier| {
            match modifier.to_ascii_uppercase().as_str() {
                "SUPER" | "MOD" | "WIN" => "Mod4",
                "ALT" => "Mod1",
                "CTRL" | "CONTROL" => "Control",
                "SHIFT" => "Shift",
                other => other,
            }
            .to_owned()
        })
        .collect();
    parts.push(key.to_owned());
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_combination_splits_into_modifiers_and_key() {
        let (modifiers, key) = split("SUPER SHIFT, L").expect("a valid combination");
        assert_eq!(modifiers, "SUPER SHIFT");
        assert_eq!(key, "L");
    }

    #[test]
    fn a_key_on_its_own_has_no_modifiers() {
        let (modifiers, key) = split("F9").expect("a valid combination");
        assert!(modifiers.is_empty());
        assert_eq!(key, "F9");
    }

    #[test]
    fn sway_spells_the_modifiers_its_own_way() {
        assert_eq!(sway_keys("SUPER SHIFT", "L"), "Mod4+Shift+L");
        assert_eq!(sway_keys("CTRL ALT", "F9"), "Control+Mod1+F9");
        assert_eq!(sway_keys("", "F9"), "F9");
    }
}
