//! Controle da janela de overlay.
//!
//! Em Wayland o cliente não decide onde fica nem se fica por cima: isso é do
//! compositor, por design do protocolo. Medido no spike (ver
//! `docs/ARCHITECTURE.md`): `set_always_on_top` é no-op, e a janela abre tiled.
//! O que funciona é pedir ao compositor via IPC.

use serde::Serialize;
use tauri::WebviewWindow;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub session_type: String,
    pub desktop: String,
    pub scale_factor: f64,
    pub is_decorated: bool,
}

#[tauri::command]
pub fn probe_environment(window: WebviewWindow) -> Result<Environment, String> {
    let env_var = |k: &str| std::env::var(k).unwrap_or_else(|_| "<vazio>".into());

    Ok(Environment {
        session_type: env_var("XDG_SESSION_TYPE"),
        desktop: env_var("XDG_CURRENT_DESKTOP"),
        scale_factor: window.scale_factor().map_err(|e| e.to_string())?,
        is_decorated: window.is_decorated().map_err(|e| e.to_string())?,
    })
}

/// Deixa o clique atravessar a janela. No Linux o tao implementa via input
/// region vazia, que é o mecanismo correto em Wayland.
#[tauri::command]
pub fn set_click_through(window: WebviewWindow, enabled: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_always_on_top(window: WebviewWindow, enabled: bool) -> Result<(), String> {
    window.set_always_on_top(enabled).map_err(|e| e.to_string())
}

/// Pede ao Hyprland o que o Wayland não deixa a janela pedir por conta própria.
#[tauri::command]
pub fn apply_hyprland_rules() -> Result<String, String> {
    let dispatches = [
        "dispatch setfloating class:^(lyricslens)$",
        "dispatch pin class:^(lyricslens)$",
    ];

    let mut log = String::new();
    for cmd in dispatches {
        let out = std::process::Command::new("hyprctl")
            .args(cmd.split_whitespace())
            .output()
            .map_err(|e| format!("falha ao executar hyprctl: {e}"))?;
        log.push_str(String::from_utf8_lossy(&out.stdout).trim());
        log.push(' ');
    }
    Ok(log.trim().to_string())
}
