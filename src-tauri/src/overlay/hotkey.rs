//! Atalho global, registrado no compositor.
//!
//! Wayland não deixa um aplicativo capturar uma combinação de teclas fora da
//! própria janela — isso é do compositor, por design. Então o app não registra
//! o atalho: ele **pede** ao Hyprland que registre, apontando de volta para o
//! próprio executável.
//!
//! O pedido aplica na hora e não escreve em arquivo nenhum. Some quando o
//! compositor reinicia — e é por isso que o app reaplica o atalho toda vez que
//! sobe, o que dá o mesmo efeito de permanência sem mexer na configuração do
//! usuário.
//!
//! Como é feito depende da versão: até a 0.54 por `hyprctl keyword bind`, da
//! 0.55 em diante por `hyprctl eval` com a API Lua, porque o `keyword` inteiro
//! deixou de existir. Quem escolhe é o mesmo `Dialeto` que o `apply_rules` já
//! usa. Ver #41.

use super::compositor::hyprland::{dialeto, hyprctl, is_hyprland, Dialeto};
use crate::i18n::UiError;

/// O comando que o compositor deve executar ao receber o atalho.
///
/// Dentro de um AppImage, `current_exe` aponta para o binário lá dentro — e ele
/// não roda sozinho: o WebKit procura seus processos auxiliares por caminhos que
/// só existem depois que o `AppRun` monta o ambiente. Um atalho apontando para
/// lá funcionaria enquanto o app já estivesse aberto (a instância única
/// repassaria o argumento) e morreria justamente quando fosse mais necessário,
/// com o app fechado.
///
/// O ponto de entrada correto é o `AppRun`, e o próprio ambiente diz onde ele
/// está.
fn comando_toggle() -> Option<String> {
    let alvo = match std::env::var("APPDIR") {
        Ok(dir) => std::path::PathBuf::from(dir).join("AppRun"),
        Err(_) => std::env::current_exe().ok()?,
    };
    Some(format!("{} toggle", alvo.display()))
}

/// Traduz a notação que o app guarda — `SUPER SHIFT, L`, a do `hyprland.conf` —
/// para a que a API Lua espera: `SUPER + SHIFT + L`.
///
/// Foi essa diferença que fez a #41 parecer sem saída. O erro do compositor
/// (*"Unknown keysym 'SUPER, F12', did you forget a +?"*) diz exatamente o que
/// falta, mas passa a impressão de que modificador e tecla são argumentos
/// separados — e `hl.bind` recusa qualquer terceiro argumento, inclusive o
/// exemplo citado na própria mensagem de erro. São dois argumentos: o combo
/// inteiro numa string só, e o dispatcher.
///
/// Serve também para o caso sem modificador, que o app permite: `", L"` vira
/// `"L"`, sem `+` sobrando nas pontas.
fn para_lua(atalho: &str) -> String {
    atalho
        .split(',')
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Escapa o que quebraria a string Lua. O comando carrega um caminho de
/// arquivo, e um diretório com aspas no nome derrubaria o `eval` inteiro.
fn lua_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn bind(atalho: &str, comando: &str) -> Result<String, String> {
    match dialeto() {
        Dialeto::Legado => hyprctl(&["keyword", "bind", &format!("{atalho},exec,{comando}")]),
        Dialeto::Lua => hyprctl(&[
            "eval",
            &format!(
                "hl.bind(\"{}\", hl.dsp.exec_cmd(\"{}\"))",
                lua_escape(&para_lua(atalho)),
                lua_escape(comando)
            ),
        ]),
    }
}

fn unbind(atalho: &str) -> Result<String, String> {
    match dialeto() {
        Dialeto::Legado => hyprctl(&["keyword", "unbind", atalho]),
        Dialeto::Lua => hyprctl(&[
            "eval",
            &format!("hl.unbind(\"{}\")", lua_escape(&para_lua(atalho))),
        ]),
    }
}

/// Troca o atalho ativo. Os dois lados podem ser vazios: `""` significa
/// "nenhum atalho".
pub fn apply(anterior: &str, novo: &str) -> Result<(), UiError> {
    if !is_hyprland() {
        return Err(UiError::new("hotkey.onlyHyprland"));
    }

    if !anterior.trim().is_empty() {
        // Se o antigo já não existir, o compositor apenas ignora.
        let _ = unbind(anterior);
    }

    if novo.trim().is_empty() {
        return Ok(());
    }

    // Sem isto, reaplicar o mesmo atalho — o que acontece a cada partida do
    // app — empilharia binds repetidos no compositor, e um toque dispararia
    // o toggle várias vezes.
    let _ = unbind(novo);

    let comando = comando_toggle().ok_or_else(|| UiError::new("hotkey.noExecutable"))?;
    let saida = bind(novo, &comando)
        .map_err(|e| UiError::new("hotkey.compositorFailed").arg("motivo", e))?;

    // Os dois dialetos respondem "ok" quando aceitam, e mandam o erro pelo
    // stdout — o `eval` no formato `error: <mensagem>`. Qualquer coisa que não
    // seja "ok" é uma combinação que o compositor não entendeu.
    if saida.trim().eq_ignore_ascii_case("ok") {
        Ok(())
    } else {
        Err(UiError::new("hotkey.refused")
            .arg("atalho", novo)
            .arg("motivo", saida.trim()))
    }
}

/// Remove o atalho — usado ao sair, para não deixar um bind apontando para um
/// executável que não está mais rodando.
pub fn clear(atalho: &str) {
    if is_hyprland() && !atalho.trim().is_empty() {
        let _ = unbind(atalho);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traduz_a_notacao_do_conf_para_a_da_api_lua() {
        assert_eq!(para_lua("SUPER, L"), "SUPER + L");
        assert_eq!(para_lua("SUPER SHIFT, L"), "SUPER + SHIFT + L");
        assert_eq!(
            para_lua("SUPER CTRL ALT SHIFT, F12"),
            "SUPER + CTRL + ALT + SHIFT + F12"
        );
    }

    /// O app aceita atalho sem modificador, e a tradução ingênua com `replace`
    /// deixaria um `" + "` na frente — que o compositor recusa como keysym.
    #[test]
    fn atalho_sem_modificador_nao_sobra_com_mais_na_ponta() {
        assert_eq!(para_lua(", L"), "L");
        assert_eq!(para_lua(",F12"), "F12");
    }

    /// Um caminho com aspas fecharia a string Lua no meio e o `eval` inteiro
    /// viraria erro de sintaxe.
    #[test]
    fn escapa_o_que_quebraria_a_string_lua() {
        assert_eq!(lua_escape(r#"/home/a"b/app"#), r#"/home/a\"b/app"#);
        assert_eq!(lua_escape(r"/home/a\b/app"), r"/home/a\\b/app");
    }
}
