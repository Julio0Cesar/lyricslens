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
    /// Margem esquerda da camada. `None` = centralizada pelo compositor.
    pub layer_margin_left: Option<i32>,
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
            layer_margin_left: s.layer_margin_left,
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

/// O monitor que serve de limite para o arraste da camada.
///
/// Cai para o primeiro monitor conhecido quando o compositor não sabe dizer
/// onde a camada está. A alternativa é `None`, e `None` significa arrastar sem
/// teto — a camada sai da tela e não volta, porque camada não é arrastável por
/// fora nem o compositor a traz de volta. Um limite aproximado é melhor que
/// limite nenhum.
fn monitor_da_camada(window: &WebviewWindow) -> Option<compositor::Retangulo> {
    monitor_do_overlay().or_else(|| monitores(window).first().copied())
}

/// O monitor cuja área contém o ponto.
fn monitor_sob(
    monitores: &[compositor::Retangulo],
    (x, y): (i32, i32),
) -> Option<compositor::Retangulo> {
    monitores
        .iter()
        .find(|&&(mx, my, mw, mh)| x >= mx && x < mx + mw && y >= my && y < my + mh)
        .copied()
}

/// Canto superior esquerdo da camada, em coordenada global.
///
/// A margem inferior é distância até a borda de baixo do monitor, então o topo
/// da camada é o que sobra depois de descontar a altura dela.
fn canto_da_camada(
    (mx, my, _, mh): compositor::Retangulo,
    geo: Geometry,
    (esq, inf): (i32, i32),
) -> (i32, i32) {
    (mx + esq, my + mh - geo.height - inf)
}

/// Margens que põem o canto da camada em `(x, y)` global.
fn margens_para_canto(
    (mx, my, _, mh): compositor::Retangulo,
    geo: Geometry,
    (x, y): (i32, i32),
) -> (i32, i32) {
    (x - mx, my + mh - geo.height - y)
}

/// Um arraste de camada em andamento.
///
/// Guarda **onde dentro da camada** o ponteiro a agarrou, e mais nada. Esse
/// ponto não muda enquanto o botão está pressionado, e é o que faz a conta ser
/// direta em vez de realimentada: a cada movimento a camada vai para
/// `cursor − agarrou`, sem depender de onde ela estava no quadro anterior.
///
/// A primeira versão somava o erro entre o cursor e o ponto agarrado, medido em
/// coordenada da própria superfície. Não dá: a superfície se move, a coordenada
/// muda junto, e não há como saber se o cursor andou ou se a camada andou. Cada
/// evento que chegasse antes de o compositor redesenhar contava o mesmo
/// deslocamento de novo, e a camada disparava para longe do cursor. Ver #35.
#[derive(Clone)]
pub struct Arraste {
    agarrou: (i32, i32),
    /// Lido uma vez, no começo. Cada consulta ao compositor custa um processo,
    /// e isto seria relido a cada movimento do ponteiro — dezenas de vezes por
    /// segundo, para uma resposta que não muda no meio de um arraste.
    monitores: Vec<compositor::Retangulo>,
}

static ARRASTE: std::sync::Mutex<Option<Arraste>> = std::sync::Mutex::new(None);

/// Começa o arraste. `false` quando o compositor não conta onde o cursor está —
/// e aí não há como arrastar a camada de forma confiável.
pub fn comecar_arraste(window: &WebviewWindow, geo: Geometry) -> bool {
    let Some(cursor) = compositor::atual().posicao_do_cursor() else {
        return false;
    };
    let Some(monitor) = monitor_da_camada(window) else {
        return false;
    };

    let margens = margens_no_monitor(margens_da_camada(geo, Some(monitor)), Some(monitor), geo);
    let (cx, cy) = canto_da_camada(monitor, geo, margens);
    *ARRASTE.lock().unwrap() = Some(Arraste {
        agarrou: (cursor.0 - cx, cursor.1 - cy),
        monitores: monitores(window),
    });
    true
}

/// Move a camada para onde o cursor está agora. Devolve as margens aplicadas e
/// o monitor em que ela ficou.
///
/// Atravessar monitores precisa de mais que aritmética de margem: uma layer
/// surface pertence a um output, e mexer na margem só a move dentro dele. Quem
/// troca é o `set_monitor` — e é por isso que o cursor precisa vir em
/// coordenada global, que atravessa os dois.
pub fn arrastar_camada(
    window: &WebviewWindow,
    geo: Geometry,
) -> Option<((i32, i32), compositor::Retangulo)> {
    let arraste = ARRASTE.lock().unwrap().clone()?;
    let cursor = compositor::atual().posicao_do_cursor()?;

    // Fora de qualquer monitor o cursor não deveria estar; se estiver, o
    // primeiro conhecido ao menos mantém a camada em algum lugar visível.
    let monitor =
        monitor_sob(&arraste.monitores, cursor).or_else(|| arraste.monitores.first().copied())?;

    let canto = (cursor.0 - arraste.agarrou.0, cursor.1 - arraste.agarrou.1);
    let margens = margens_no_monitor(margens_para_canto(monitor, geo, canto), Some(monitor), geo);

    layer_shell::mover_margens(window, monitor, margens);
    Some((margens, monitor))
}

pub fn terminar_arraste() {
    *ARRASTE.lock().unwrap() = None;
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

/// Onde a camada está agora, em margens `(esquerda, inferior)`.
///
/// O arraste precisa de um ponto de partida explícito, e "centralizado" não é
/// um: enquanto a margem esquerda é `None` quem escolhe a posição é o
/// compositor, e não há de onde somar deslocamento. Aqui esse estado vira o
/// número que ele produziria, e a partir do primeiro arraste a posição passa a
/// ser do usuário.
fn margens_da_camada(geo: Geometry, monitor: Option<compositor::Retangulo>) -> (i32, i32) {
    let esquerda = geo.layer_margin_left.unwrap_or_else(|| {
        monitor.map_or(0, |(_, _, largura, _)| (largura - geo.width).max(0) / 2)
    });
    (esquerda, geo.margin_bottom)
}

/// Prende as margens ao monitor.
///
/// Uma janela comum que escapa da tela ainda pode ser trazida de volta pelo
/// compositor, e por isso `posicao_visivel` se contenta com 48px sobrando. A
/// camada não tem esse resgate: ela não é arrastável por fora, o compositor não
/// a move, e o único jeito de recuperá-la seria editar o `settings.json` na
/// mão. Então o limite aqui é rígido — a camada inteira fica dentro do monitor.
fn margens_no_monitor(
    (esquerda, inferior): (i32, i32),
    monitor: Option<compositor::Retangulo>,
    geo: Geometry,
) -> (i32, i32) {
    // Sem saber o tamanho do monitor não há teto em que confiar; o chão em zero
    // já impede o caso que importa, que é a camada sair pela borda de baixo ou
    // da esquerda.
    let Some((_, _, largura, altura)) = monitor else {
        return (esquerda.max(0), inferior.max(0));
    };
    (
        esquerda.clamp(0, (largura - geo.width).max(0)),
        inferior.clamp(0, (altura - geo.height).max(0)),
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

    fn geo_camada(layer_margin_left: Option<i32>) -> Geometry {
        Geometry {
            width: 780,
            height: 160,
            margin_bottom: 80,
            position: None,
            layer_margin_left,
            blur: true,
            layer_shell: true,
        }
    }

    const MONITOR: Option<compositor::Retangulo> = Some((0, 0, 1920, 1080));

    /// Sem margem gravada quem centraliza é o compositor, e o arraste não tem
    /// de onde partir. O ponto de partida é o número que ele produziria.
    #[test]
    fn arraste_parte_do_centro_quando_nao_ha_margem_gravada() {
        let (esquerda, inferior) = margens_da_camada(geo_camada(None), MONITOR);
        assert_eq!(esquerda, (1920 - 780) / 2);
        assert_eq!(inferior, 80);
    }

    #[test]
    fn margem_gravada_vence_o_centro() {
        let (esquerda, _) = margens_da_camada(geo_camada(Some(42)), MONITOR);
        assert_eq!(esquerda, 42);
    }

    /// Sem monitor conhecido não há como centralizar, e um palpite poria a
    /// camada num lugar arbitrário. Zero é a borda, que ao menos é visível.
    #[test]
    fn sem_monitor_conhecido_a_camada_parte_da_borda() {
        let (esquerda, _) = margens_da_camada(geo_camada(None), None);
        assert_eq!(esquerda, 0);
    }

    /// A camada não tem resgate: fora da tela, só editando o arquivo na mão.
    #[test]
    fn arrastar_alem_da_borda_para_na_borda() {
        let geo = geo_camada(None);
        assert_eq!(margens_no_monitor((-500, -500), MONITOR, geo), (0, 0));
        assert_eq!(
            margens_no_monitor((99_999, 99_999), MONITOR, geo),
            (1920 - 780, 1080 - 160)
        );
    }

    #[test]
    fn dentro_do_monitor_o_arraste_passa_intacto() {
        assert_eq!(
            margens_no_monitor((300, 200), MONITOR, geo_camada(None)),
            (300, 200)
        );
    }

    /// A margem inferior é distância até a borda de baixo: o topo da camada é o
    /// que sobra depois de descontar a altura dela. Um sinal trocado aqui faz a
    /// camada fugir do cursor em vez de segui-lo.
    #[test]
    fn converte_margem_em_canto_e_de_volta() {
        let geo = geo_camada(None);
        let monitor = (0, 0, 1920, 1080);
        // 1080 - 160 (altura) - 80 (margem inferior) = 840
        assert_eq!(canto_da_camada(monitor, geo, (300, 80)), (300, 840));
        assert_eq!(margens_para_canto(monitor, geo, (300, 840)), (300, 80));
    }

    /// No segundo monitor a margem é distância até a **borda dele**, não até a
    /// origem do desktop. Confundir os dois joga a camada um monitor inteiro
    /// para o lado — foi o que aconteceu quando a margem gravada de um monitor
    /// foi aplicada com a origem do outro.
    #[test]
    fn a_margem_e_relativa_ao_monitor_nao_ao_desktop() {
        let geo = geo_camada(None);
        let segundo = (1920, 0, 1920, 1080);
        assert_eq!(canto_da_camada(segundo, geo, (300, 80)), (2220, 840));
        assert_eq!(margens_para_canto(segundo, geo, (2220, 840)), (300, 80));
    }

    #[test]
    fn acha_o_monitor_sob_o_cursor() {
        let m = [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
        assert_eq!(monitor_sob(&m, (10, 10)), Some(m[0]));
        assert_eq!(monitor_sob(&m, (2500, 500)), Some(m[1]));
        // A borda pertence ao monitor da direita, não aos dois.
        assert_eq!(monitor_sob(&m, (1920, 0)), Some(m[1]));
        assert_eq!(monitor_sob(&m, (1919, 0)), Some(m[0]));
        // Fora de tudo — o cursor não deveria chegar aqui, mas se chegar não
        // pode virar um monitor inventado.
        assert_eq!(monitor_sob(&m, (99_999, 0)), None);
    }

    /// Sem monitor não há teto — e foi exatamente isso que soltou a camada da
    /// tela na primeira vez que este arraste rodou de verdade: em modo camada o
    /// overlay não aparece em `hyprctl clients`, porque camada não é janela, e
    /// quem perguntava o monitor recebia `None`. O piso em zero é o que sobra;
    /// quem evita chegar aqui é o `monitor_da_camada`, que cai para o primeiro
    /// monitor conhecido antes de desistir.
    #[test]
    fn sem_monitor_sobra_o_piso_e_nada_mais() {
        let geo = geo_camada(None);
        assert_eq!(margens_no_monitor((-50, -50), None, geo), (0, 0));
        assert_eq!(margens_no_monitor((99_999, 5), None, geo), (99_999, 5));
    }

    /// Uma camada mais larga que o monitor não pode gerar um teto negativo — o
    /// `clamp` entraria em pânico com o mínimo acima do máximo.
    #[test]
    fn camada_maior_que_o_monitor_nao_derruba_o_limite() {
        let mut geo = geo_camada(None);
        geo.width = 3000;
        geo.height = 2000;
        assert_eq!(margens_no_monitor((10, 10), MONITOR, geo), (0, 0));
    }

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
