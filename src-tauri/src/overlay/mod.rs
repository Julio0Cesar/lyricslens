//! Controle da janela de overlay.
//!
//! Em Wayland o cliente não decide onde fica nem se fica por cima: isso é do
//! compositor, por design do protocolo. Medido no spike (ver
//! `docs/ARCHITECTURE.md`): `set_always_on_top` é no-op e a janela abre tiled.
//! O que funciona é pedir ao compositor via IPC.

use std::time::Duration;

use serde::Serialize;
use tauri::{Manager, WebviewWindow};

pub const OVERLAY_LABEL: &str = "overlay";

/// Tamanho e folga do rodapé usados ao recolocar o overlay.
const LARGURA: i32 = 780;
const ALTURA: i32 = 300;
const MARGEM_INFERIOR: i32 = 80;

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

pub fn is_hyprland() -> bool {
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
}

fn hyprctl(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|e| format!("hyprctl indisponível: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Pede ao compositor o que o Wayland não deixa a janela pedir por conta
/// própria: flutuar, ficar fixa em todas as áreas de trabalho, e assumir
/// tamanho e posição escolhidos.
#[tauri::command]
pub fn apply_compositor_rules(window: WebviewWindow) -> Result<String, String> {
    if !is_hyprland() {
        return Ok("compositor sem regras conhecidas — janela fica como o sistema decidir".into());
    }

    let seletor = "class:^(lyricslens)$";
    hyprctl(&["dispatch", "setfloating", seletor])?;
    hyprctl(&["dispatch", "pin", seletor])?;
    hyprctl(&[
        "dispatch",
        "resizewindowpixel",
        &format!("exact {LARGURA} {ALTURA},{seletor}"),
    ])?;

    // Canto inferior central do monitor onde a janela está.
    if let Ok(Some(monitor)) = window.current_monitor() {
        let pos = monitor.position();
        let size = monitor.size();
        let x = pos.x + (size.width as i32 - LARGURA) / 2;
        let y = pos.y + size.height as i32 - ALTURA - MARGEM_INFERIOR;
        hyprctl(&[
            "dispatch",
            "movewindowpixel",
            &format!("exact {x} {y},{seletor}"),
        ])?;
    }

    Ok("regras aplicadas".into())
}

/// As regras só pegam depois que o compositor conhece a janela. Em vez de um
/// `sleep` no escuro, espera ela aparecer na lista de clientes.
pub async fn apply_rules_when_mapped(window: WebviewWindow) {
    if !is_hyprland() {
        return;
    }

    for _ in 0..25 {
        if hyprctl(&["clients"]).is_ok_and(|s| s.contains("class: lyricslens")) {
            if let Err(e) = apply_compositor_rules(window) {
                eprintln!("[overlay] regras do compositor falharam: {e}");
            }
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    eprintln!("[overlay] a janela não apareceu no compositor a tempo");
}

/// Mostra o overlay e o recoloca — ao reaparecer, o compositor pode tê-lo
/// devolvido para o layout em mosaico.
pub fn show(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let _ = window.show();
    let _ = window.set_focus();

    let w = window.clone();
    tauri::async_runtime::spawn(apply_rules_when_mapped(w));
}

pub fn hide(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = window.hide();
    }
}

/// O que o ícone da bandeja e o atalho global chamam.
pub fn toggle(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        hide(app);
    } else {
        show(app);
    }
}

#[tauri::command]
pub fn toggle_overlay(app: tauri::AppHandle) {
    toggle(&app);
}
