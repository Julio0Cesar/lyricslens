//! Hyprland, via `hyprctl`.
//!
//! Era a única implementação do app e vivia solta no `overlay/mod.rs`. Aqui ela
//! é só mais um `Compositor` — o que permitiu o Sway entrar sem mexer em nada
//! do que já funcionava. Ver #12.

use std::sync::OnceLock;

use super::{Compositor, Retangulo, NAMESPACE, TITULO};
use crate::i18n::UiError;
use crate::overlay::hotkey;

/// O seletor que casa **só** a janela do overlay.
///
/// Tem que ser por título: a janela de configurações é do mesmo processo e da
/// mesma classe, e sem isso as regras pegariam as duas.
const SELETOR: &str = "title:^(LyricsLens Overlay)$";

pub struct Hyprland;

pub fn rodando() -> bool {
    is_hyprland()
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

/// Qual sintaxe de `dispatch` o compositor desta máquina entende.
///
/// O Hyprland 0.55 trocou o parser do `hyprctl dispatch` por Lua. A sintaxe de
/// string — `dispatch setfloating title:^(...)$` — deixou de existir e passou a
/// devolver erro de sintaxe **de Lua**, não "dispatcher inválido". Como o app
/// lia só o stdout e não checava o conteúdo, ele seguia achando que tinha dado
/// certo: o resultado é o overlay parar de flutuar, de ser fixado e de assumir
/// tamanho e posição, em silêncio, depois de uma atualização do compositor.
///
/// O mesmo corte vale para os atalhos: o `keyword` inteiro deixou de existir na
/// 0.55, e não só para `bind` — `hyprctl keyword` de qualquer coisa responde
/// *"keyword can't work with non-legacy parsers. Use eval."*. Ver #41.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Dialeto {
    /// `dispatch setfloating title:^(...)$`
    Legado,
    /// `dispatch hl.dsp.window.float{ window = hl.get_window("...") }`
    Lua,
}

pub(crate) fn dialeto() -> Dialeto {
    static CACHE: OnceLock<Dialeto> = OnceLock::new();
    *CACHE.get_or_init(|| {
        // Sonda sem efeito colateral nos dois lados: o seletor não casa com
        // janela nenhuma, então o Hyprland antigo responde "ok" sem fazer nada
        // e o novo falha no parser antes de chegar a agir.
        let sonda = hyprctl(&["dispatch", "setfloating", "title:^(__lyricslens_probe__)$"])
            .unwrap_or_default();
        let d = if sonda.starts_with("error:") || sonda.contains("hl.dispatch") {
            Dialeto::Lua
        } else {
            Dialeto::Legado
        };
        crate::log::logar!(Info, "overlay", "hyprctl no dialeto {d:?}");
        d
    })
}

/// A janela do overlay, na forma que o dialeto Lua espera receber.
fn alvo_lua() -> String {
    format!("window = hl.get_window(\"{SELETOR}\")")
}

fn flutuar() -> Result<String, String> {
    match dialeto() {
        Dialeto::Legado => hyprctl(&["dispatch", "setfloating", SELETOR]),
        // Na API Lua `float` é **toggle**, não setter: chamar numa janela que
        // já flutua a devolve para o mosaico. Quem garante a idempotência é o
        // chamador, lendo o estado antes.
        Dialeto::Lua => hyprctl(&[
            "dispatch",
            &format!("hl.dsp.window.float{{ {} }}", alvo_lua()),
        ]),
    }
}

fn fixar() -> Result<String, String> {
    match dialeto() {
        Dialeto::Legado => hyprctl(&["dispatch", "pin", SELETOR]),
        // `pin` também é toggle.
        Dialeto::Lua => hyprctl(&[
            "dispatch",
            &format!("hl.dsp.window.pin{{ {} }}", alvo_lua()),
        ]),
    }
}

fn redimensionar(largura: i32, altura: i32) -> Result<String, String> {
    match dialeto() {
        Dialeto::Legado => hyprctl(&[
            "dispatch",
            "resizewindowpixel",
            &format!("exact {largura} {altura},{SELETOR}"),
        ]),
        Dialeto::Lua => hyprctl(&[
            "dispatch",
            &format!(
                "hl.dsp.window.resize{{ x = {largura}, y = {altura}, {} }}",
                alvo_lua()
            ),
        ]),
    }
}

fn mover(x: i32, y: i32) -> Result<String, String> {
    match dialeto() {
        Dialeto::Legado => hyprctl(&[
            "dispatch",
            "movewindowpixel",
            &format!("exact {x} {y},{SELETOR}"),
        ]),
        Dialeto::Lua => hyprctl(&[
            "dispatch",
            &format!("hl.dsp.window.move{{ x = {x}, y = {y}, {} }}", alvo_lua()),
        ]),
    }
}

/// O nome da regra de desfoque.
///
/// É o que permite desfazer sem `hyprctl reload`: reaplicar com o valor
/// invertido substitui a regra, e o efeito pega em janela já aberta. A #18
/// tinha registrado o contrário — que só a sintaxe descontinuada era aceita e
/// que desfazer exigiria recarregar a configuração inteira do usuário. As duas
/// coisas valiam para o `keyword`, que morreu na 0.55; pela API Lua nenhuma das
/// duas vale.
const REGRA_DESFOQUE: &str = "lyricslens-blur";

/// Liga e desliga o desfoque do que está atrás do overlay.
///
/// São duas regras porque são dois objetos diferentes para o compositor, com
/// APIs e até com polaridade diferentes: janela comum é `window_rule` com
/// `no_blur` (negativo, casando por título), camada é `layer_rule` com `blur`
/// (positivo, casando por namespace). Aplicar as duas sempre sai mais barato do
/// que descobrir em qual modo o overlay está: a que não corresponde ao modo
/// atual simplesmente não casa com nada.
fn definir_desfoque(atras: bool) -> Result<(), String> {
    let checar = |saida: String| -> Result<(), String> {
        if saida.trim().eq_ignore_ascii_case("ok") {
            Ok(())
        } else {
            Err(saida.trim().to_string())
        }
    };

    checar(hyprctl(&[
        "eval",
        &format!(
            "hl.window_rule({{ name = \"{REGRA_DESFOQUE}-janela\", \
             match = {{ title = \"^({TITULO})$\" }}, no_blur = {} }})",
            !atras
        ),
    ])?)?;

    checar(hyprctl(&[
        "eval",
        &format!(
            "hl.layer_rule({{ name = \"{REGRA_DESFOQUE}-camada\", \
             match = {{ namespace = \"^({NAMESPACE})$\" }}, blur = {atras} }})"
        ),
    ])?)
}

/// `"607, 881"` → `(607, 881)`.
fn ler_cursorpos(saida: &str) -> Option<(i32, i32)> {
    let (x, y) = saida.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn cliente_overlay() -> Option<serde_json::Value> {
    let json = hyprctl(&["-j", "clients"]).ok()?;
    let clients: serde_json::Value = serde_json::from_str(&json).ok()?;
    clients
        .as_array()?
        .iter()
        .find(|c| c.get("title").and_then(|t| t.as_str()) == Some("LyricsLens Overlay"))
        .cloned()
}

/// O nome do monitor onde a **camada** do overlay está.
///
/// Uma layer surface não aparece em `hyprctl clients`: aquilo lista janelas, e
/// camada não é janela. Sem isto o modo camada não tinha monitor conhecido, e
/// quem perguntava recebia `None` — o que fez o limite do arraste cair no ramo
/// "sem teto" e deixou a camada escapar da tela, de onde não há como trazê-la
/// de volta. Ver #35.
fn monitor_da_camada() -> Option<String> {
    let json = hyprctl(&["-j", "layers"]).ok()?;
    let layers: serde_json::Value = serde_json::from_str(&json).ok()?;

    layers.as_object()?.iter().find_map(|(monitor, dados)| {
        let tem = dados
            .get("levels")?
            .as_object()?
            .values()
            .filter_map(|nivel| nivel.as_array())
            .flatten()
            .any(|l| l.get("namespace").and_then(|n| n.as_str()) == Some(NAMESPACE));
        tem.then(|| monitor.clone())
    })
}

/// Geometria `(x, y, largura, altura)` do monitor onde o overlay está.
///
/// Pergunta pelos dois caminhos porque o overlay pode ser das duas naturezas:
/// janela comum, que está em `clients`, ou camada, que está em `layers`.
fn monitor_do_overlay() -> Option<(i32, i32, i32, i32)> {
    if !is_hyprland() {
        return None;
    }

    let id = cliente_overlay().and_then(|c| c.get("monitor")?.as_i64());
    let nome = if id.is_none() {
        monitor_da_camada()
    } else {
        None
    };
    if id.is_none() && nome.is_none() {
        return None;
    }

    let json = hyprctl(&["-j", "monitors"]).ok()?;
    let monitors: serde_json::Value = serde_json::from_str(&json).ok()?;
    let m = monitors.as_array()?.iter().find(|m| match (id, &nome) {
        (Some(id), _) => m.get("id").and_then(|i| i.as_i64()) == Some(id),
        (None, Some(nome)) => m.get("name").and_then(|n| n.as_str()) == Some(nome.as_str()),
        _ => false,
    })?;

    Some((
        m.get("x")?.as_i64()? as i32,
        m.get("y")?.as_i64()? as i32,
        m.get("width")?.as_i64()? as i32,
        m.get("height")?.as_i64()? as i32,
    ))
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

impl Compositor for Hyprland {
    fn nome(&self) -> &'static str {
        "hyprland"
    }

    fn preparar(&self, largura: i32, altura: i32) -> Result<(), String> {
        // Flutuar e fixar são *toggles* no dialeto Lua, então só são chamados
        // quando o estado atual não é o desejado. Sem isto, reaplicar as
        // regras — o que acontece a cada `show` e a cada mudança de geometria —
        // desfaria o que a aplicação anterior fez.
        let estado = cliente_overlay();
        let bool_de = |campo: &str| {
            estado
                .as_ref()
                .and_then(|c| c.get(campo))
                .and_then(|v| v.as_bool())
        };

        if bool_de("floating") != Some(true) {
            flutuar()?;
        }
        if bool_de("pinned") != Some(true) {
            fixar()?;
        }
        redimensionar(largura, altura)?;
        Ok(())
    }

    fn mover(&self, x: i32, y: i32) -> Result<(), String> {
        mover(x, y)?;
        Ok(())
    }

    fn monitores(&self) -> Option<Vec<Retangulo>> {
        monitores_do_compositor()
    }

    fn monitor_do_overlay(&self) -> Option<Retangulo> {
        monitor_do_overlay()
    }

    fn posicao_atual(&self) -> Option<(i32, i32)> {
        let at = cliente_overlay()?;
        let at = at.get("at")?.as_array()?;
        Some((at.first()?.as_i64()? as i32, at.get(1)?.as_i64()? as i32))
    }

    fn janela_conhecida(&self) -> bool {
        hyprctl(&["clients"]).is_ok_and(|s| s.contains(TITULO))
    }

    fn posicao_do_cursor(&self) -> Option<(i32, i32)> {
        ler_cursorpos(&hyprctl(&["cursorpos"]).ok()?)
    }

    /// Só pela API Lua. No dialeto legado a única sintaxe aceita já era a
    /// descontinuada, e desfazer uma regra individual exigiria `hyprctl
    /// reload` — recarregar a configuração inteira da sessão do usuário por
    /// causa de uma preferência do overlay. Melhor não oferecer o controle do
    /// que oferecê-lo com esse preço escondido.
    fn sabe_desfocar(&self) -> bool {
        dialeto() == Dialeto::Lua
    }

    fn definir_desfoque(&self, atras: bool) -> Result<(), String> {
        if !self.sabe_desfocar() {
            return Err("precisa do Hyprland 0.55 ou mais novo".into());
        }
        definir_desfoque(atras)
    }

    fn registrar_atalho(&self, anterior: &str, novo: &str) -> Result<(), UiError> {
        hotkey::apply(anterior, novo)
    }

    fn limpar_atalho(&self, atalho: &str) {
        hotkey::clear(atalho);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_a_posicao_do_cursor() {
        assert_eq!(ler_cursorpos("607, 881"), Some((607, 881)));
        assert_eq!(ler_cursorpos("0,0"), Some((0, 0)));
        // Segundo monitor: a coordenada é global e atravessa os dois.
        assert_eq!(ler_cursorpos("2516, 661"), Some((2516, 661)));
    }

    /// O `hyprctl` pode responder erro em vez de coordenada. Um `unwrap` aqui
    /// derrubaria o app no meio de um arraste.
    #[test]
    fn resposta_que_nao_e_coordenada_nao_vira_posicao() {
        assert_eq!(ler_cursorpos(""), None);
        assert_eq!(ler_cursorpos("error: unknown request"), None);
        assert_eq!(ler_cursorpos("607"), None);
        assert_eq!(ler_cursorpos("a, b"), None);
    }
}
