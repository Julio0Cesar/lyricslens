//! Iniciar com a sessão.
//!
//! O app foi desenhado para viver em segundo plano — bandeja, atalho global,
//! instância única. Precisar abrir na mão a cada sessão desfaz boa parte disso.
//!
//! O mecanismo é o do freedesktop: um `.desktop` em `~/.config/autostart/`, que
//! todo ambiente de mesa relevante lê. Não há daemon, não há serviço de usuário,
//! não há nada para desinstalar depois — o estado *é* a existência do arquivo,
//! e é dele que a interface lê. Assim a opção nunca mostra ligado o que não
//! está: se o usuário apagar o arquivo por fora, o app concorda na próxima vez
//! que olhar.

use std::path::PathBuf;

/// Onde o executável está, do ponto de vista de quem vai chamá-lo depois.
///
/// Dentro de um AppImage, `current_exe` aponta para o binário lá dentro — e ele
/// não roda sozinho: o WebKit procura seus processos auxiliares por caminhos que
/// só existem depois que o `AppRun` monta o ambiente. Um autostart apontando
/// para lá falharia exatamente no cenário em que ele existe, com o app fechado.
/// É a mesma armadilha do atalho global (ver `overlay::hotkey`).
fn executavel() -> Option<PathBuf> {
    match std::env::var("APPDIR") {
        Ok(dir) => Some(PathBuf::from(dir).join("AppRun")),
        Err(_) => std::env::current_exe().ok(),
    }
}

fn diretorio() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join("autostart"))
}

fn arquivo() -> Option<PathBuf> {
    Some(diretorio()?.join("lyricslens.desktop"))
}

/// Um argumento do `Exec=` na forma que a especificação do freedesktop manda.
///
/// Argumentos são separados por espaço, então um caminho com espaço vira dois
/// argumentos e o comando não roda. A especificação resolve com aspas duplas, e
/// dentro delas `"`, `` ` ``, `$` e `\` precisam de contrabarra. Não é caso
/// hipotético: `XDG_DATA_HOME` é do usuário e pode apontar para onde ele quiser.
fn citar(arg: &str) -> String {
    if !arg.contains([' ', '\t', '"', '`', '$', '\\', '\'']) {
        return arg.to_string();
    }
    let mut s = String::with_capacity(arg.len() + 2);
    s.push('"');
    for c in arg.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            s.push('\\');
        }
        s.push(c);
    }
    s.push('"');
    s
}

/// O `.desktop` que a sessão vai executar.
///
/// `hide` é o argumento que já existe e faz o app subir direto para a bandeja.
/// Sem ele, entrar na sessão jogaria o overlay na cara do usuário toda vez.
fn conteudo(exec: &std::path::Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=LyricsLens\n\
         Comment=Letras sincronizadas sobre qualquer aplicativo\n\
         Exec={} hide\n\
         Icon=lyricslens\n\
         Terminal=false\n\
         Categories=AudioVideo;Audio;Music;\n\
         X-GNOME-Autostart-enabled=true\n",
        citar(&exec.display().to_string())
    )
}

pub fn is_enabled() -> bool {
    arquivo().is_some_and(|p| p.is_file())
}

pub fn set_enabled(ligar: bool) -> Result<(), String> {
    let alvo = arquivo().ok_or("não descobri onde fica a sua pasta de configuração")?;
    let exec = executavel().ok_or("não descobri o caminho do executável")?;
    aplicar(&alvo, &exec, ligar)
}

/// O trabalho de verdade, com o destino explícito para poder ser testado sem
/// mexer no `~/.config` de quem roda os testes.
fn aplicar(alvo: &std::path::Path, exec: &std::path::Path, ligar: bool) -> Result<(), String> {
    if !ligar {
        // Já não existir não é erro: o que importa é o estado final.
        return match std::fs::remove_file(alvo) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("não consegui remover {}: {e}", alvo.display())),
        };
    }

    if let Some(pai) = alvo.parent() {
        std::fs::create_dir_all(pai)
            .map_err(|e| format!("não consegui criar {}: {e}", pai.display()))?;
    }
    std::fs::write(alvo, conteudo(exec))
        .map_err(|e| format!("não consegui escrever {}: {e}", alvo.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_desktop_sobe_escondido() {
        let texto = conteudo(std::path::Path::new("/usr/bin/lyricslens"));
        assert!(
            texto.contains("Exec=/usr/bin/lyricslens hide"),
            "sem `hide` o overlay aparece na cara do usuário a cada login"
        );
    }

    #[test]
    fn o_desktop_tem_o_cabecalho_que_a_sessao_espera() {
        let texto = conteudo(std::path::Path::new("/usr/bin/lyricslens"));
        assert!(texto.starts_with("[Desktop Entry]\n"));
        assert!(texto.contains("Type=Application\n"));
        assert!(texto.contains("Terminal=false\n"));
        assert!(
            texto.ends_with('\n'),
            "arquivo .desktop termina com newline"
        );
    }

    fn linha_exec(caminho: &str) -> String {
        conteudo(std::path::Path::new(caminho))
            .lines()
            .find(|l| l.starts_with("Exec="))
            .expect("tem linha Exec")
            .to_string()
    }

    /// Caminho com espaço não pode quebrar o `Exec` em dois argumentos — a
    /// sessão tentaria executar `/home/ana` e falharia. `XDG_DATA_HOME` é do
    /// usuário e pode apontar para onde ele quiser.
    #[test]
    fn caminho_com_espaco_vai_entre_aspas() {
        assert_eq!(
            linha_exec("/home/ana maria/.local/bin/lyricslens"),
            "Exec=\"/home/ana maria/.local/bin/lyricslens\" hide"
        );
    }

    #[test]
    fn caminho_comum_nao_ganha_aspas_a_toa() {
        assert_eq!(
            linha_exec("/usr/bin/lyricslens"),
            "Exec=/usr/bin/lyricslens hide"
        );
    }

    /// Dentro das aspas, a especificação exige contrabarra em `"`, `` ` ``,
    /// `$` e `\`.
    #[test]
    fn caracteres_especiais_sao_escapados_dentro_das_aspas() {
        assert_eq!(
            linha_exec("/home/x/pasta $com \"aspas\"/lyricslens"),
            "Exec=\"/home/x/pasta \\$com \\\"aspas\\\"/lyricslens\" hide"
        );
    }

    /// Ligar, desligar e ligar de novo, escrevendo de verdade em disco.
    #[test]
    fn liga_desliga_e_religa_criando_a_pasta_se_faltar() {
        let base =
            std::env::temp_dir().join(format!("lyricslens-autostart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        // A pasta `autostart` costuma não existir numa sessão nova.
        let alvo = base.join("autostart").join("lyricslens.desktop");
        let exec = std::path::Path::new("/usr/bin/lyricslens");

        assert!(!alvo.exists());

        aplicar(&alvo, exec, true).unwrap();
        assert!(alvo.is_file(), "criou a pasta e o arquivo");
        let texto = std::fs::read_to_string(&alvo).unwrap();
        assert!(texto.contains("Exec=/usr/bin/lyricslens hide"));

        aplicar(&alvo, exec, false).unwrap();
        assert!(!alvo.exists(), "desligar remove o arquivo");

        // Desligar o que já está desligado não é erro.
        aplicar(&alvo, exec, false).unwrap();

        aplicar(&alvo, exec, true).unwrap();
        assert!(alvo.is_file(), "religar recria");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Ligar duas vezes não pode duplicar nem corromper o arquivo.
    #[test]
    fn ligar_de_novo_apenas_reescreve() {
        let base =
            std::env::temp_dir().join(format!("lyricslens-autostart-idem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let alvo = base.join("lyricslens.desktop");
        let exec = std::path::Path::new("/usr/bin/lyricslens");

        aplicar(&alvo, exec, true).unwrap();
        let primeiro = std::fs::read_to_string(&alvo).unwrap();
        aplicar(&alvo, exec, true).unwrap();
        let segundo = std::fs::read_to_string(&alvo).unwrap();

        assert_eq!(primeiro, segundo);
        assert_eq!(segundo.matches("[Desktop Entry]").count(), 1);

        let _ = std::fs::remove_dir_all(&base);
    }
}
