//! Sway, e os outros que falam o IPC do i3.
//!
//! O `swaymsg` tem o equivalente de tudo o que o app pede ao `hyprctl`, com uma
//! diferença que muda o desenho: **a notação de atalho é outra**. O Hyprland
//! quer `SUPER SHIFT, L`; o Sway quer `Mod4+Shift+l`. A captura na interface
//! produz a do Hyprland, então aqui há uma tradução — e ela é pura, logo
//! testável sem compositor nenhum.

use super::{Compositor, Retangulo, TITULO};
use crate::i18n::UiError;

pub struct Sway;

/// O `SWAYSOCK` é o sinal confiável: ele só existe com o Sway rodando, e existe
/// mesmo quando o `XDG_CURRENT_DESKTOP` diz outra coisa — o que acontece em
/// sessão iniciada por gerenciador de login.
pub fn rodando() -> bool {
    std::env::var_os("SWAYSOCK").is_some_and(|v| !v.is_empty())
}

fn swaymsg(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("swaymsg")
        .args(args)
        .output()
        .map_err(|e| format!("swaymsg indisponível: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Roda um comando no critério que casa só a janela do overlay.
///
/// O critério precisa ser por título: a janela de configurações é do mesmo
/// processo e da mesma classe, e sem isso as regras pegariam as duas.
fn no_overlay(comando: &str) -> Result<String, String> {
    swaymsg(&[&format!("[title=\"^{TITULO}$\"] {comando}")])
}

fn consultar(o_que: &str) -> Option<serde_json::Value> {
    serde_json::from_str(&swaymsg(&["-t", o_que, "-r"]).ok()?).ok()
}

/// Procura a janela do overlay na árvore, que é recursiva.
fn achar_no<'a>(no: &'a serde_json::Value, alvo: &str) -> Option<&'a serde_json::Value> {
    if no.get("name").and_then(|n| n.as_str()) == Some(alvo) {
        return Some(no);
    }
    for chave in ["nodes", "floating_nodes"] {
        for filho in no.get(chave)?.as_array()? {
            if let Some(achado) = achar_no(filho, alvo) {
                return Some(achado);
            }
        }
    }
    None
}

fn retangulo(v: &serde_json::Value) -> Option<Retangulo> {
    let r = v.get("rect")?;
    Some((
        r.get("x")?.as_i64()? as i32,
        r.get("y")?.as_i64()? as i32,
        r.get("width")?.as_i64()? as i32,
        r.get("height")?.as_i64()? as i32,
    ))
}

/// `SUPER SHIFT, L` → `Mod4+Shift+l`.
///
/// A captura na interface fala a notação do Hyprland porque foi o primeiro
/// compositor suportado. Traduzir aqui — e não mudar a captura — mantém o
/// `settings.json` portátil: o mesmo atalho gravado funciona nos dois.
pub fn atalho_do_hyprland(atalho: &str) -> Option<String> {
    let (mods, tecla) = atalho.split_once(',')?;
    let tecla = tecla.trim();
    if tecla.is_empty() {
        return None;
    }

    let mut partes: Vec<String> = mods
        .split_whitespace()
        .map(|m| match m.to_uppercase().as_str() {
            "SUPER" => "Mod4".to_string(),
            "CTRL" | "CONTROL" => "Ctrl".to_string(),
            "ALT" => "Mod1".to_string(),
            "SHIFT" => "Shift".to_string(),
            outro => outro.to_string(),
        })
        .collect();

    // O Sway espera o keysym do X11: minúscula para letra, nome próprio para
    // as especiais — que é justamente o que a captura já grava para elas.
    partes.push(if tecla.chars().count() == 1 {
        tecla.to_lowercase()
    } else {
        tecla.to_string()
    });

    Some(partes.join("+"))
}

fn comando_toggle() -> Option<String> {
    let alvo = match std::env::var("APPDIR") {
        Ok(dir) => std::path::PathBuf::from(dir).join("AppRun"),
        Err(_) => std::env::current_exe().ok()?,
    };
    Some(format!("{} toggle", alvo.display()))
}

impl Compositor for Sway {
    fn nome(&self) -> &'static str {
        "sway"
    }

    fn preparar(&self, largura: i32, altura: i32) -> Result<(), String> {
        // `enable` é idempotente no Sway, ao contrário do `toggle` do dialeto
        // Lua do Hyprland — aqui não é preciso consultar o estado antes.
        no_overlay("floating enable")?;
        no_overlay("sticky enable")?;
        no_overlay(&format!("resize set {largura} {altura}"))?;
        Ok(())
    }

    fn mover(&self, x: i32, y: i32) -> Result<(), String> {
        no_overlay(&format!("move absolute position {x} {y}"))?;
        Ok(())
    }

    fn monitores(&self) -> Option<Vec<Retangulo>> {
        Some(
            consultar("get_outputs")?
                .as_array()?
                .iter()
                .filter(|o| o.get("active").and_then(|a| a.as_bool()) != Some(false))
                .filter_map(retangulo)
                .collect(),
        )
    }

    fn monitor_do_overlay(&self) -> Option<Retangulo> {
        let (x, y) = self.posicao_atual()?;
        // O Sway não diz em qual saída a janela está; descobre-se por conter.
        self.monitores()?
            .into_iter()
            .find(|(mx, my, mw, mh)| x >= *mx && x < mx + mw && y >= *my && y < my + mh)
    }

    fn posicao_atual(&self) -> Option<(i32, i32)> {
        let arvore = consultar("get_tree")?;
        let (x, y, _, _) = retangulo(achar_no(&arvore, TITULO)?)?;
        Some((x, y))
    }

    fn janela_conhecida(&self) -> bool {
        consultar("get_tree").is_some_and(|a| achar_no(&a, TITULO).is_some())
    }

    fn registrar_atalho(&self, anterior: &str, novo: &str) -> Result<(), UiError> {
        if !anterior.trim().is_empty() {
            self.limpar_atalho(anterior);
        }
        if novo.trim().is_empty() {
            return Ok(());
        }

        let combo = atalho_do_hyprland(novo).ok_or_else(|| {
            UiError::new("hotkey.refused")
                .arg("atalho", novo)
                .arg("motivo", "combinação incompleta")
        })?;
        let comando = comando_toggle().ok_or_else(|| UiError::new("hotkey.noExecutable"))?;

        swaymsg(&[&format!("bindsym {combo} exec {comando}")]).map_err(|e| {
            UiError::new("hotkey.refused")
                .arg("atalho", novo)
                .arg("motivo", e)
        })?;
        Ok(())
    }

    fn limpar_atalho(&self, atalho: &str) {
        if let Some(combo) = atalho_do_hyprland(atalho) {
            let _ = swaymsg(&[&format!("unbindsym {combo}")]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traduz_modificadores_para_a_notacao_do_sway() {
        assert_eq!(atalho_do_hyprland("SUPER, L").as_deref(), Some("Mod4+l"));
        assert_eq!(
            atalho_do_hyprland("CTRL SHIFT, K").as_deref(),
            Some("Ctrl+Shift+k")
        );
        assert_eq!(atalho_do_hyprland("ALT, F1").as_deref(), Some("Mod1+F1"));
    }

    /// Letra vira minúscula porque o Sway espera keysym do X11; tecla especial
    /// mantém o nome, que é o que a captura já grava.
    #[test]
    fn letra_vira_minuscula_e_especial_mantem_o_nome() {
        assert_eq!(atalho_do_hyprland("SUPER, L").as_deref(), Some("Mod4+l"));
        assert_eq!(atalho_do_hyprland("SUPER, up").as_deref(), Some("Mod4+up"));
        assert_eq!(
            atalho_do_hyprland("SUPER, space").as_deref(),
            Some("Mod4+space")
        );
    }

    #[test]
    fn atalho_sem_tecla_nao_vira_bind() {
        assert_eq!(atalho_do_hyprland("SUPER, "), None);
        assert_eq!(atalho_do_hyprland("sem virgula"), None);
        assert_eq!(atalho_do_hyprland(""), None);
    }

    /// Sem modificador é atalho válido no Sway — e sequestraria a tecla da
    /// sessão inteira, mas quem decide isso é o usuário, não o app.
    #[test]
    fn tecla_sozinha_ainda_produz_bind() {
        assert_eq!(atalho_do_hyprland(", F12").as_deref(), Some("F12"));
    }

    #[test]
    fn acha_a_janela_no_meio_da_arvore() {
        let arvore = serde_json::json!({
            "name": "root",
            "nodes": [{
                "name": "saida",
                "nodes": [],
                "floating_nodes": [{
                    "name": TITULO,
                    "rect": { "x": 100, "y": 200, "width": 780, "height": 160 },
                    "nodes": [], "floating_nodes": []
                }]
            }],
            "floating_nodes": []
        });
        let no = achar_no(&arvore, TITULO).expect("devia achar");
        assert_eq!(retangulo(no), Some((100, 200, 780, 160)));
    }

    #[test]
    fn arvore_sem_a_janela_nao_inventa() {
        let arvore = serde_json::json!({
            "name": "root", "nodes": [], "floating_nodes": []
        });
        assert!(achar_no(&arvore, TITULO).is_none());
    }
}
