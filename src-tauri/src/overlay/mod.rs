//! Controle da janela de overlay.
//!
//! Em Wayland o cliente não decide onde fica nem se fica por cima: isso é do
//! compositor, por design do protocolo. Medido no spike (ver
//! `docs/ARCHITECTURE.md`): `set_always_on_top` é no-op e a janela abre tiled.
//! O que funciona é pedir ao compositor via IPC.

pub mod compositor;
pub mod hotkey;
pub mod layer_shell;

use std::time::Duration;

use crate::log::logar;
use serde::Serialize;
use tauri::{LogicalPosition, LogicalSize, Manager, WebviewWindow};

/// O seletor tem que ser por título: a janela de configurações é do mesmo
/// processo e da mesma classe (`Lyricslens`), e deve continuar sendo uma janela
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
    /// Borrar o que está atrás do overlay. Quem borra é o compositor.
    pub blur: bool,
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
            blur: s.blur,
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

/// Pede ao compositor o que o Wayland não deixa a janela pedir por conta
/// própria: flutuar, ficar fixa em todas as áreas de trabalho, e assumir
/// tamanho e posição escolhidos.
///
/// O seletor tem que ser específico: sem `title:`, a regra pegaria também a
/// janela de configurações, que é do mesmo processo e da mesma classe.
pub fn apply_rules(window: &WebviewWindow, geo: Geometry) -> Result<String, String> {
    // Antes do desvio da camada, porque vale nos dois modos. Falhar aqui não
    // derruba o resto: é uma preferência de aparência, e um compositor sem
    // controle de desfoque não impede o overlay de ser posto no lugar.
    if let Err(e) = compositor::atual().definir_desfoque(geo.blur) {
        logar!(Info, "overlay", "sem controle de desfoque: {e}");
    }

    if geo.layer_shell {
        layer_shell::update_geometry(window, geo);
        return Ok("posição vem do protocolo de camadas".into());
    }

    // O tamanho é a única parte da geometria que o cliente pode pedir em
    // Wayland, e o caminho de janela comum simplesmente nunca a pedia: os
    // campos `width` e `height` do settings.json só surtiam efeito no caminho
    // da camada, e mexer nos sliders não mudava nada. Ver #36.
    let _ = window.set_size(LogicalSize::new(geo.width as f64, geo.height as f64));

    let c = compositor::atual();

    if !c.sabe_posicionar() {
        // Em X11 o `set_position` do Tauri funciona de verdade; em Wayland
        // desconhecido o compositor ignora, e não há o que fazer. A mensagem
        // diz isso em vez de fingir que a janela foi posta em algum lugar.
        if let Some((x, y)) = posicao_desejada(window, geo) {
            let _ = window.set_position(LogicalPosition::new(x as f64, y as f64));
        }
        return Ok("compositor sem regras conhecidas — janela fica como o sistema decidir".into());
    }

    c.preparar(geo.width, geo.height)?;
    if let Some((x, y)) = posicao_desejada(window, geo) {
        c.mover(x, y)?;
    }

    Ok(format!("regras aplicadas ({})", c.nome()))
}

/// Geometria `(x, y, largura, altura)` do monitor onde o overlay está.
fn monitor_do_overlay() -> Option<(i32, i32, i32, i32)> {
    compositor::atual().monitor_do_overlay()
}

/// Geometria de todos os monitores, `(x, y, largura, altura)`.
///
/// Prefere o compositor: a janela nasce oculta para o layer-shell poder ser
/// inicializado antes de ela existir, e nesse ponto o Tauri ainda não conhece
/// monitor nenhum.
fn monitores(window: &WebviewWindow) -> Vec<(i32, i32, i32, i32)> {
    if let Some(lista) = compositor::atual().monitores() {
        if !lista.is_empty() {
            return lista;
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
        logar!(
            Aviso,
            "overlay",
            "a posição guardada ({}, {}) não cai em nenhum monitor — recentralizando",
            xy.0,
            xy.1
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

/// Onde a janela está agora, segundo o compositor.
///
/// Em Wayland o cliente não sabe a própria posição — o protocolo não conta.
/// Quem sabe é o compositor, então é a ele que se pergunta.
pub fn current_position() -> Option<(i32, i32)> {
    compositor::atual().posicao_atual()
}

/// As regras só pegam depois que o compositor conhece a janela. Em vez de um
/// `sleep` no escuro, espera ela aparecer na lista de clientes.
pub async fn apply_rules_when_mapped(window: WebviewWindow, geo: Geometry) {
    let c = compositor::atual();
    if geo.layer_shell || !c.sabe_posicionar() {
        return;
    }

    for _ in 0..25 {
        if c.janela_conhecida() {
            if let Err(e) = apply_rules(&window, geo) {
                logar!(Aviso, "overlay", "regras do compositor falharam: {e}");
            }
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    logar!(
        Aviso,
        "overlay",
        "a janela não apareceu no compositor a tempo"
    );
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
    logar!(Aviso, "overlay", "janela de configurações não encontrada");
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
