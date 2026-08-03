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
use tauri::{LogicalPosition, LogicalSize, Manager, WebviewWindow};

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

    // O tamanho é a única parte da geometria que o cliente pode pedir em
    // Wayland, e o caminho de janela comum simplesmente nunca a pedia: os
    // campos `width` e `height` do settings.json só surtiam efeito no caminho
    // da camada, e mexer nos sliders não mudava nada. Ver #36.
    let _ = window.set_size(LogicalSize::new(geo.width as f64, geo.height as f64));

    if !is_hyprland() {
        // Em X11 isto funciona de verdade; em Wayland o compositor ignora, e
        // aí não há o que fazer sem uma implementação por compositor (#12).
        if let Some((x, y)) = posicao_desejada(window, geo) {
            let _ = window.set_position(LogicalPosition::new(x as f64, y as f64));
        }
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

    let destino = posicao_desejada(window, geo);

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

/// Geometria de todos os monitores, `(x, y, largura, altura)`.
///
/// Prefere o compositor: a janela nasce oculta para o layer-shell poder ser
/// inicializado antes de ela existir, e nesse ponto o Tauri ainda não conhece
/// monitor nenhum.
fn monitores(window: &WebviewWindow) -> Vec<(i32, i32, i32, i32)> {
    if is_hyprland() {
        if let Some(list) = monitores_do_compositor() {
            if !list.is_empty() {
                return list;
            }
        }
    }
    window
        .available_monitors()
        .map(|ms| {
            ms.iter()
                .map(|m| {
                    let (p, s) = (m.position(), m.size());
                    (p.x, p.y, s.width as i32, s.height as i32)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn monitores_do_compositor() -> Option<Vec<(i32, i32, i32, i32)>> {
    let json = hyprctl(&["-j", "monitors"]).ok()?;
    let monitors: serde_json::Value = serde_json::from_str(&json).ok()?;
    Some(
        monitors
            .as_array()?
            .iter()
            .filter_map(|m| {
                Some((
                    m.get("x")?.as_i64()? as i32,
                    m.get("y")?.as_i64()? as i32,
                    m.get("width")?.as_i64()? as i32,
                    m.get("height")?.as_i64()? as i32,
                ))
            })
            .collect(),
    )
}

/// Quanto da janela precisa sobrar dentro de um monitor para ela ainda ser
/// alcançável com o mouse.
const VISIVEL_MINIMO: i32 = 48;

/// A posição guardada ainda cai em algum monitor que existe?
///
/// `position_x`/`position_y` são coordenadas absolutas, que atravessam todos os
/// monitores. Desconectar um monitor externo, ou só mudar a disposição deles,
/// faz essas coordenadas apontarem para o nada — e obedecer a elas some com a
/// janela sem o usuário ter como trazê-la de volta, porque não dá para arrastar
/// o que não se vê. Ver #13.
fn posicao_visivel(monitores: &[(i32, i32, i32, i32)], (x, y): (i32, i32), w: i32, h: i32) -> bool {
    // Sem saber onde estão os monitores, não há base para discordar do que
    // está gravado.
    if monitores.is_empty() {
        return true;
    }
    let minimo_x = VISIVEL_MINIMO.min(w);
    let minimo_y = VISIVEL_MINIMO.min(h);
    monitores.iter().any(|&(mx, my, mw, mh)| {
        let sobreposicao_x = (x + w).min(mx + mw) - x.max(mx);
        let sobreposicao_y = (y + h).min(my + mh) - y.max(my);
        sobreposicao_x >= minimo_x && sobreposicao_y >= minimo_y
    })
}

/// Onde a janela deve ficar: a posição que o usuário escolheu, ou o rodapé
/// central do monitor. Uma posição guardada que não cabe mais em tela nenhuma é
/// descartada em silêncio em favor do rodapé — o alternativo é a janela sumir.
fn posicao_desejada(window: &WebviewWindow, geo: Geometry) -> Option<(i32, i32)> {
    let monitores = monitores(window);

    if let Some(xy) = geo.position {
        if posicao_visivel(&monitores, xy, geo.width, geo.height) {
            return Some(xy);
        }
        eprintln!(
            "[overlay] a posição guardada ({}, {}) não cai em nenhum monitor — recentralizando",
            xy.0, xy.1
        );
    }

    monitor_do_overlay()
        .or_else(|| monitores.first().copied())
        .map(|(mx, my, mw, mh)| {
            (
                mx + (mw - geo.width) / 2,
                my + mh - geo.height - geo.margin_bottom,
            )
        })
}

/// Regras registradas *antes* de a janela ser mapeada.
///
/// O `dispatch setfloating` corrige depois do fato: a janela chega a nascer
/// tilada, reorganiza todas as outras do workspace e ocupa metade da tela até
/// alguém consertar. Uma `windowrule` já está valendo no instante em que o
/// compositor mapeia a janela, então ela nunca chega a ser tilada. Ver #32.
pub fn register_window_rules() {
    if !is_hyprland() {
        return;
    }
    for regra in ["float", "pin", "noborder"] {
        // O seletor casa pelo título porque a classe (`Lyricslens`) é a mesma
        // da janela de configurações, que deve continuar sendo uma janela
        // normal.
        let valor = format!("{regra}, title:^(LyricsLens Overlay)$");

        // O Hyprland 0.56 depreciou `windowrulev2` e passou a aceitar a
        // sintaxe v2 direto no `windowrule`; versões anteriores só entendem
        // essa sintaxe no `windowrulev2`. Tentar os dois cobre as duas.
        let saida = hyprctl(&["keyword", "windowrule", &valor]).unwrap_or_default();
        if saida.to_lowercase().contains("invalid") {
            let _ = hyprctl(&["keyword", "windowrulev2", &valor]);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Dois monitores 1920×1080 lado a lado, o da direita começando em x=1920.
    const DOIS_MONITORES: &[(i32, i32, i32, i32)] = &[(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
    const OVERLAY: (i32, i32) = (760, 260);

    #[test]
    fn posicao_dentro_do_monitor_principal_e_visivel() {
        assert!(posicao_visivel(
            DOIS_MONITORES,
            (500, 800),
            OVERLAY.0,
            OVERLAY.1
        ));
    }

    #[test]
    fn posicao_no_monitor_secundario_e_visivel() {
        assert!(posicao_visivel(
            DOIS_MONITORES,
            (2400, 500),
            OVERLAY.0,
            OVERLAY.1
        ));
    }

    /// O caso do #13: a janela ficou no monitor externo, o monitor saiu, e a
    /// coordenada guardada agora aponta para o nada.
    #[test]
    fn posicao_no_monitor_desconectado_nao_e_visivel() {
        let so_o_principal = &[(0, 0, 1920, 1080)];
        assert!(!posicao_visivel(
            so_o_principal,
            (2400, 500),
            OVERLAY.0,
            OVERLAY.1
        ));
    }

    #[test]
    fn posicao_negativa_fora_de_qualquer_monitor_nao_e_visivel() {
        assert!(!posicao_visivel(
            DOIS_MONITORES,
            (-2000, -2000),
            OVERLAY.0,
            OVERLAY.1
        ));
    }

    /// Encostada na borda direita, mas com uma faixa ainda agarrável.
    #[test]
    fn sobra_de_margem_na_borda_ainda_conta_como_visivel() {
        let so_o_principal = &[(0, 0, 1920, 1080)];
        assert!(posicao_visivel(
            so_o_principal,
            (1920 - VISIVEL_MINIMO, 500),
            OVERLAY.0,
            OVERLAY.1
        ));
        assert!(!posicao_visivel(
            so_o_principal,
            (1920 - VISIVEL_MINIMO + 1, 500),
            OVERLAY.0,
            OVERLAY.1
        ));
    }

    /// Sem lista de monitores não há base para discordar do que está gravado —
    /// melhor obedecer do que recentralizar por engano.
    #[test]
    fn sem_monitores_conhecidos_confia_no_que_esta_gravado() {
        assert!(posicao_visivel(&[], (99999, 99999), OVERLAY.0, OVERLAY.1));
    }

    /// Janela menor que a folga mínima não pode ser considerada invisível só
    /// por ser pequena.
    #[test]
    fn janela_menor_que_a_folga_minima_usa_o_proprio_tamanho() {
        let so_o_principal = &[(0, 0, 1920, 1080)];
        assert!(posicao_visivel(so_o_principal, (100, 100), 20, 10));
    }
}
