//! O que cada compositor sabe fazer.
//!
//! Em Wayland a janela não decide onde fica nem se fica por cima — quem decide
//! é o compositor, e cada um fala um protocolo próprio. Até aqui o app sabia
//! conversar com um só, o Hyprland; em qualquer outro ambiente a janela abria
//! onde o sistema mandasse e o botão "recolocar" não fazia nada. Ver #12.
//!
//! O `trait` aqui é o mesmo desenho do `MediaProvider`: um compositor novo é um
//! arquivo novo, não uma mudança no app.
//!
//! Quando nenhum é reconhecido, o `Nenhum` responde honestamente — `None` para
//! toda consulta e `false` em `sabe_posicionar`, e é isso que faz a interface
//! dizer que não há caminho em vez de oferecer um botão que não faz nada.

use crate::i18n::UiError;

pub mod hyprland;
pub mod sway;

/// Como a janela do overlay é identificada em qualquer compositor.
pub const TITULO: &str = "LyricsLens Overlay";

/// Como a **camada** do overlay é identificada.
///
/// Título não serve aqui: uma layer surface não tem título, e nem aparece na
/// lista de janelas do compositor. O que ela tem é o namespace, declarado na
/// criação da superfície.
pub const NAMESPACE: &str = "lyricslens";

/// Geometria de um monitor: `(x, y, largura, altura)`.
pub type Retangulo = (i32, i32, i32, i32);

pub trait Compositor: Send + Sync {
    fn nome(&self) -> &'static str;

    /// Este compositor sabe pôr a janela onde o app pede?
    ///
    /// `false` não é falha — é a resposta honesta que a interface precisa para
    /// não prometer o que não pode cumprir.
    fn sabe_posicionar(&self) -> bool {
        true
    }

    /// Flutuar, fixar em todas as áreas de trabalho e assumir o tamanho.
    fn preparar(&self, largura: i32, altura: i32) -> Result<(), String>;

    fn mover(&self, x: i32, y: i32) -> Result<(), String>;

    /// Todos os monitores. `None` quando o compositor não sabe responder.
    fn monitores(&self) -> Option<Vec<Retangulo>>;

    /// O monitor onde o overlay está.
    fn monitor_do_overlay(&self) -> Option<Retangulo>;

    /// Onde o overlay está agora — é o que permite lembrar a posição depois de
    /// o usuário arrastar.
    fn posicao_atual(&self) -> Option<(i32, i32)>;

    /// O compositor já conhece a janela? As regras só pegam depois disso.
    fn janela_conhecida(&self) -> bool;

    /// Onde o ponteiro está, em coordenada global.
    ///
    /// Wayland não conta isso ao cliente: uma janela só recebe posição relativa
    /// à própria superfície. Isso basta para clicar, e não basta para arrastar
    /// uma camada — quando a superfície se move, a coordenada relativa muda
    /// sozinha, e não há como distinguir "o cursor andou" de "a camada andou".
    /// Deduzir pela diferença faz a camada disparar na frente do cursor.
    ///
    /// Quem sabe a verdade é o compositor. `None` quando ele não conta, e aí o
    /// arraste da camada não é oferecido. Ver #35.
    fn posicao_do_cursor(&self) -> Option<(i32, i32)> {
        None
    }

    /// Este compositor sabe ligar e desligar o desfoque atrás do overlay?
    ///
    /// Separado do `definir_desfoque` porque a interface precisa saber disso
    /// *antes* de oferecer o controle — um botão que não faz nada é pior que
    /// botão nenhum.
    fn sabe_desfocar(&self) -> bool {
        false
    }

    /// Borrar ou não o que está **atrás** do overlay.
    ///
    /// Quem borra o fundo de uma janela é o compositor, não o app: o
    /// `backdrop-filter` do CSS borra o que está atrás do elemento *dentro da
    /// página*, e atrás desta página não há página nenhuma — há o desktop, que
    /// é do compositor. Ver #18.
    fn definir_desfoque(&self, _atras: bool) -> Result<(), String> {
        Err("este compositor não tem controle de desfoque conhecido".into())
    }

    /// Troca o atalho global. Vazio dos dois lados significa "nenhum".
    fn registrar_atalho(&self, anterior: &str, novo: &str) -> Result<(), UiError>;

    fn limpar_atalho(&self, atalho: &str);
}

/// Nenhum compositor reconhecido.
///
/// Em X11 o `set_always_on_top` e o `set_position` do próprio Tauri funcionam,
/// e é o `apply_rules` que os usa quando chega aqui. Em Wayland desconhecido
/// não há o que fazer, e dizer isso é melhor que fingir.
pub struct Nenhum;

impl Compositor for Nenhum {
    fn nome(&self) -> &'static str {
        "desconhecido"
    }
    fn sabe_posicionar(&self) -> bool {
        false
    }
    fn preparar(&self, _l: i32, _a: i32) -> Result<(), String> {
        Ok(())
    }
    fn mover(&self, _x: i32, _y: i32) -> Result<(), String> {
        Ok(())
    }
    fn monitores(&self) -> Option<Vec<Retangulo>> {
        None
    }
    fn monitor_do_overlay(&self) -> Option<Retangulo> {
        None
    }
    fn posicao_atual(&self) -> Option<(i32, i32)> {
        None
    }
    fn janela_conhecida(&self) -> bool {
        // Sem IPC não há como saber; dizer que sim evita uma espera inútil.
        true
    }
    fn registrar_atalho(&self, _anterior: &str, _novo: &str) -> Result<(), UiError> {
        Err(UiError::new("hotkey.semCompositor"))
    }
    fn limpar_atalho(&self, _atalho: &str) {}
}

/// Quem está rodando.
///
/// A ordem importa: o Hyprland também define `XDG_CURRENT_DESKTOP`, então a
/// checagem específica de cada um vem antes de qualquer heurística geral.
pub fn atual() -> &'static dyn Compositor {
    static CACHE: std::sync::OnceLock<Box<dyn Compositor>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let c: Box<dyn Compositor> = if hyprland::rodando() {
                Box::new(hyprland::Hyprland)
            } else if sway::rodando() {
                Box::new(sway::Sway)
            } else {
                Box::new(Nenhum)
            };
            crate::log::escrever(
                crate::log::Nivel::Info,
                "overlay",
                &format!("compositor: {}", c.nome()),
            );
            c
        })
        .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_desconhecido_nao_promete_o_que_nao_cumpre() {
        let c = Nenhum;
        assert!(!c.sabe_posicionar());
        assert!(c.monitores().is_none());
        assert!(c.posicao_atual().is_none());
        assert!(c.registrar_atalho("", "SUPER, L").is_err());
    }

    /// Preparar e mover não podem *falhar* no desconhecido: quem decide o que
    /// fazer sem compositor é o `apply_rules`, e um erro aqui viraria mensagem
    /// de falha para uma situação que é só ausência de recurso.
    #[test]
    fn o_desconhecido_nao_erra_ao_ser_chamado() {
        let c = Nenhum;
        assert!(c.preparar(800, 200).is_ok());
        assert!(c.mover(0, 0).is_ok());
    }
}
