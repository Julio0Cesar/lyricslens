//! SPIKE: investigação das capacidades de overlay em Wayland/Hyprland.
//!
//! Este arquivo é descartável. O objetivo é responder, com dado real:
//!   1. `transparent: true` produz janela realmente transparente?
//!   2. `set_always_on_top` tem algum efeito em Wayland?
//!   3. `set_ignore_cursor_events` produz click-through de verdade?
//!   4. Regras do Hyprland (float/pin) resolvem o que o Wayland proíbe?

use serde::Serialize;
use tauri::{Manager, WebviewWindow};

#[derive(Serialize)]
struct Environment {
    session_type: String,
    desktop: String,
    wayland_display: String,
    scale_factor: f64,
    outer_size: (u32, u32),
    is_decorated: bool,
}

/// Coleta o ambiente de janela como o Tauri o enxerga.
#[tauri::command]
fn probe_environment(window: WebviewWindow) -> Result<Environment, String> {
    let env_var = |k: &str| std::env::var(k).unwrap_or_else(|_| "<vazio>".into());

    Ok(Environment {
        session_type: env_var("XDG_SESSION_TYPE"),
        desktop: env_var("XDG_CURRENT_DESKTOP"),
        wayland_display: env_var("WAYLAND_DISPLAY"),
        scale_factor: window.scale_factor().map_err(|e| e.to_string())?,
        outer_size: {
            let s = window.outer_size().map_err(|e| e.to_string())?;
            (s.width, s.height)
        },
        is_decorated: window.is_decorated().map_err(|e| e.to_string())?,
    })
}

/// Click-through. No Linux o tao implementa via input region vazia
/// (`gtk_widget_input_shape_combine_region`), que é exatamente o mecanismo
/// correto em Wayland — o compositor deixa o clique passar para a janela de baixo.
#[tauri::command]
fn set_click_through(window: WebviewWindow, enabled: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|e| e.to_string())
}

/// Em Wayland isso costuma virar no-op: o protocolo não permite que o cliente
/// se coloque acima dos outros. Retorna Ok mesmo quando nada acontece — por isso
/// a verificação real é visual + `hyprctl clients`.
#[tauri::command]
fn set_always_on_top(window: WebviewWindow, enabled: bool) -> Result<(), String> {
    window.set_always_on_top(enabled).map_err(|e| e.to_string())
}

/// Aplica as regras do Hyprland na janela em runtime, via IPC do compositor.
/// É o caminho alternativo quando o protocolo Wayland recusa o pedido do cliente.
#[tauri::command]
fn apply_hyprland_rules() -> Result<String, String> {
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
        log.push_str(&format!(
            "{cmd} -> {}",
            String::from_utf8_lossy(&out.stdout).trim()
        ));
        log.push('\n');
    }
    Ok(log)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            probe_environment,
            set_click_through,
            set_always_on_top,
            apply_hyprland_rules
        ])
        .setup(|app| {
            // Log de partida: útil para confirmar o backend de janela em uso.
            if let Some(w) = app.get_webview_window("overlay") {
                println!("[spike] janela 'overlay' criada; decorada={:?}", w.is_decorated());
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
