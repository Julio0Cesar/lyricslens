//! Atalho global, registrado no compositor.
//!
//! Wayland não deixa um aplicativo capturar uma combinação de teclas fora da
//! própria janela — isso é do compositor, por design. Então o app não registra
//! o atalho: ele **pede** ao Hyprland que registre, apontando de volta para o
//! próprio executável.
//!
//! O pedido é feito por `hyprctl keyword`, que aplica na hora e não escreve em
//! arquivo nenhum. Some quando o compositor reinicia — e é por isso que o app
//! reaplica o atalho toda vez que sobe, o que dá o mesmo efeito de permanência
//! sem mexer na configuração do usuário.

use super::{hyprctl, is_hyprland};

/// Como o app se identifica ao ser chamado pelo atalho.
fn comando_toggle() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("{} toggle", exe.display()))
}

/// Troca o atalho ativo. Os dois lados podem ser vazios: `""` significa
/// "nenhum atalho".
pub fn apply(anterior: &str, novo: &str) -> Result<(), String> {
    if !is_hyprland() {
        return Err("atalho automático só em Hyprland — use o keybind do seu compositor".into());
    }

    if !anterior.trim().is_empty() {
        // Se o antigo já não existir, o compositor apenas ignora.
        let _ = hyprctl(&["keyword", "unbind", anterior]);
    }

    if novo.trim().is_empty() {
        return Ok(());
    }

    // Sem isto, reaplicar o mesmo atalho — o que acontece a cada partida do
    // app — empilharia binds repetidos no compositor, e um toque dispararia
    // o toggle várias vezes.
    let _ = hyprctl(&["keyword", "unbind", novo]);

    let comando = comando_toggle().ok_or("não descobri o caminho do executável")?;
    let saida = hyprctl(&["keyword", "bind", &format!("{novo},exec,{comando}")])?;

    // `hyprctl keyword` responde "ok" quando aceita; qualquer outra coisa é
    // uma combinação que o compositor não entendeu.
    if saida.trim().eq_ignore_ascii_case("ok") {
        Ok(())
    } else {
        Err(format!("o compositor recusou \"{novo}\": {saida}"))
    }
}

/// Remove o atalho — usado ao sair, para não deixar um bind apontando para um
/// executável que não está mais rodando.
pub fn clear(atalho: &str) {
    if is_hyprland() && !atalho.trim().is_empty() {
        let _ = hyprctl(&["keyword", "unbind", atalho]);
    }
}
