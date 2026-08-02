//! Controle da janela de overlay.
//!
//! Em Wayland o cliente não decide onde fica nem se fica por cima: isso é do
//! compositor, por design do protocolo. Medido no spike (ver
//! `docs/ARCHITECTURE.md`): `set_always_on_top` é no-op e a janela abre tiled.
//! O que funciona é pedir ao compositor via IPC.

pub mod hotkey;
pub mod layer_shell;

use std::time::Duration;

use serde::Serialize;
use tauri::{Manager, WebviewWindow};

pub const OVERLAY_LABEL: &str = "overlay";
pub const SETTINGS_LABEL: &str = "settings";

/// Tamanho e folga do rodapé, vindos das preferências do usuário.
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub width: i32,
    pub height: i32,
    pub margin_bottom: i32,
    /// Onde o usuário largou a janela. `None` recentraliza no rodapé.
    pub position: Option<(i32, i32)>,
    /// Quando a janela é uma camada do compositor, quem posiciona é o
    /// protocolo — as regras via IPC não se aplicam e atrapalhariam.
    pub layer_shell: bool,
}

impl From<&crate::store::Settings> for Geometry {
    fn from(s: &crate::store::Settings) -> Self {
        Self {
            width: s.width as i32,
            height: s.height as i32,
            margin_bottom: s.margin_bottom as i32,
            position: s.position_x.zip(s.position_y),
            layer_shell: s.layer_shell,
        }
    }
}

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

pub(crate) fn hyprctl(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|e| format!("hyprctl indisponível: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Pede ao compositor o que o Wayland não deixa a janela pedir por conta
/// própria: flutuar, ficar fixa em todas as áreas de trabalho, e assumir
/// tamanho e posição escolhidos.
///
/// O seletor tem que ser específico: sem `title:`, a regra pegaria também a
/// janela de configurações, que é do mesmo processo e da mesma classe.
pub fn apply_rules(window: &WebviewWindow, geo: Geometry) -> Result<String, String> {
    if geo.layer_shell {
        layer_shell::update_geometry(window, geo);
        return Ok("posição vem do protocolo de camadas".into());
    }
    if !is_hyprland() {
        return Ok("compositor sem regras conhecidas — janela fica como o sistema decidir".into());
    }

    let seletor = "title:^(LyricsLens Overlay)$";
    hyprctl(&["dispatch", "setfloating", seletor])?;
    hyprctl(&["dispatch", "pin", seletor])?;
    hyprctl(&[
        "dispatch",
        "resizewindowpixel",
        &format!("exact {} {},{seletor}", geo.width, geo.height),
    ])?;

    // Posição escolhida pelo usuário; sem ela, rodapé central do monitor.
    //
    // A geometria do monitor vem do compositor, não do Tauri: a janela nasce
    // oculta para o layer-shell poder ser inicializado antes de ela existir, e
    // nesse ponto o `current_monitor()` ainda responde vazio. Quem sempre sabe
    // é o compositor.
    let destino = match geo.position {
        Some(xy) => Some(xy),
        None => monitor_do_overlay()
            .or_else(|| {
                let m = window.current_monitor().ok().flatten()?;
                let (p, s) = (m.position(), m.size());
                Some((p.x, p.y, s.width as i32, s.height as i32))
            })
            .map(|(mx, my, mw, mh)| {
                (
                    mx + (mw - geo.width) / 2,
                    my + mh - geo.height - geo.margin_bottom,
                )
            }),
    };

    if let Some((x, y)) = destino {
        hyprctl(&[
            "dispatch",
            "movewindowpixel",
            &format!("exact {x} {y},{seletor}"),
        ])?;
    }

    Ok("regras aplicadas".into())
}

/// Nossa janela na lista de clientes do compositor.
fn cliente_overlay() -> Option<serde_json::Value> {
    let json = hyprctl(&["-j", "clients"]).ok()?;
    let clients: serde_json::Value = serde_json::from_str(&json).ok()?;
    clients
        .as_array()?
        .iter()
        .find(|c| c.get("title").and_then(|t| t.as_str()) == Some("LyricsLens Overlay"))
        .cloned()
}

/// Geometria `(x, y, largura, altura)` do monitor onde o overlay está.
fn monitor_do_overlay() -> Option<(i32, i32, i32, i32)> {
    if !is_hyprland() {
        return None;
    }
    let id = cliente_overlay()?.get("monitor")?.as_i64()?;

    let json = hyprctl(&["-j", "monitors"]).ok()?;
    let monitors: serde_json::Value = serde_json::from_str(&json).ok()?;
    let m = monitors
        .as_array()?
        .iter()
        .find(|m| m.get("id").and_then(|i| i.as_i64()) == Some(id))?;

    Some((
        m.get("x")?.as_i64()? as i32,
        m.get("y")?.as_i64()? as i32,
        m.get("width")?.as_i64()? as i32,
        m.get("height")?.as_i64()? as i32,
    ))
}

/// Onde a janela está agora, segundo o compositor.
///
/// Em Wayland o cliente não sabe a própria posição — o protocolo não conta.
/// Quem sabe é o compositor, então é a ele que se pergunta.
pub fn current_position() -> Option<(i32, i32)> {
    if !is_hyprland() {
        return None;
    }
    let at = cliente_overlay()?;
    let at = at.get("at")?.as_array()?;
    Some((at.first()?.as_i64()? as i32, at.get(1)?.as_i64()? as i32))
}

/// As regras só pegam depois que o compositor conhece a janela. Em vez de um
/// `sleep` no escuro, espera ela aparecer na lista de clientes.
pub async fn apply_rules_when_mapped(window: WebviewWindow, geo: Geometry) {
    if geo.layer_shell || !is_hyprland() {
        return;
    }

    for _ in 0..25 {
        if hyprctl(&["clients"]).is_ok_and(|s| s.contains("LyricsLens Overlay")) {
            if let Err(e) = apply_rules(&window, geo) {
                eprintln!("[overlay] regras do compositor falharam: {e}");
            }
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    eprintln!("[overlay] a janela não apareceu no compositor a tempo");
}

/// Fontes instaladas no sistema, para o seletor de fonte.
#[tauri::command]
pub fn list_fonts() -> Vec<String> {
    let Ok(out) = std::process::Command::new("fc-list")
        .args(["--format", "%{family[0]}\n"])
        .output()
    else {
        return Vec::new();
    };

    let mut nomes: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();

    nomes.sort_unstable();
    nomes.dedup();
    nomes
}

/// Mostra o overlay e o recoloca — ao reaparecer, o compositor pode tê-lo
/// devolvido para o layout em mosaico.
pub fn show(app: &tauri::AppHandle, geo: Geometry) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let _ = window.show();
    let _ = window.set_focus();

    tauri::async_runtime::spawn(apply_rules_when_mapped(window, geo));
}

/// Abre a janela de configurações, criando-a se ainda não existir.
pub fn open_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    eprintln!("[overlay] janela de configurações não encontrada");
}

pub fn close_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = window.hide();
    }
}

pub fn hide(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = window.hide();
    }
}

/// O que o ícone da bandeja e o atalho global chamam.
pub fn toggle(app: &tauri::AppHandle, geo: Geometry) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        hide(app);
    } else {
        show(app, geo);
    }
}
