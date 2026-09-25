//! The words the person reads, in their language.
//!
//! Keyed by the English string rather than by a symbol: the code stays
//! readable without chasing a table to find out what a screen says, and a
//! sentence with no translation falls back to the one written here.
//!
//! Code, comments and commits stay in English. This is only what is on screen.

/// The languages there are words for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Portuguese,
}

/// What the session asks for, from the usual variables.
pub fn language() -> Language {
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        let Ok(value) = std::env::var(name) else {
            continue;
        };
        if value.to_ascii_lowercase().starts_with("pt") {
            return Language::Portuguese;
        }
        if !value.is_empty() && value != "C" && value != "POSIX" {
            return Language::English;
        }
    }
    Language::English
}

/// The sentence to show, translated when there is a translation.
pub fn t(english: &str) -> String {
    if language() == Language::English {
        return english.to_owned();
    }
    PORTUGUESE
        .iter()
        .find(|(key, _)| *key == english)
        .map_or_else(
            || english.to_owned(),
            |(_, translated)| (*translated).to_owned(),
        )
}

/// Every sentence the program puts on screen, and its Portuguese.
const PORTUGUESE: &[(&str, &str)] = &[
    // The tray menu.
    ("Show / hide", "Mostrar ou esconder"),
    ("Move the overlay", "Mover o overlay"),
    ("Preferences…", "Preferências…"),
    ("Quit", "Sair"),
    // The preferences window, by section.
    ("Player", "Player"),
    (
        "Which player to follow when more than one is open.",
        "Qual player seguir quando há mais de um aberto.",
    ),
    ("Part of the bus name", "Parte do nome no barramento"),
    ("Lyrics for this track", "Letra desta faixa"),
    (
        "When the wrong words are on screen, find the right ones by hand.",
        "Quando a letra na tela está errada, encontre a certa à mão.",
    ),
    ("Artist", "Artista"),
    ("Title", "Título"),
    ("Search", "Buscar"),
    ("Searching…", "Buscando…"),
    (
        "A title is the least it needs.",
        "É preciso ao menos um título.",
    ),
    ("The overlay is not running.", "O overlay não está aberto."),
    (
        "Nothing found under that name.",
        "Nada encontrado com esse nome.",
    ),
    ("Now showing these", "Mostrando esta agora"),
    ("Appearance", "Aparência"),
    (
        "Every change here shows on the overlay straight away.",
        "Toda mudança aqui aparece no overlay na hora.",
    ),
    ("Font size", "Tamanho da fonte"),
    ("Font", "Fonte"),
    ("Font weight", "Peso da fonte"),
    ("Line up the words", "Alinhamento das palavras"),
    ("Centre", "Centro"),
    ("Left", "Esquerda"),
    ("Right", "Direita"),
    ("Width", "Largura"),
    (
        "In pixels. Where a long line wraps",
        "Em pixels. Onde uma linha longa quebra",
    ),
    ("Rounded corners", "Cantos arredondados"),
    (
        "In pixels, on the strip behind the line",
        "Em pixels, na faixa atrás da linha",
    ),
    ("Distance from the bottom", "Distância da base"),
    ("In pixels", "Em pixels"),
    ("Text colour", "Cor do texto"),
    ("Shadow under the text", "Sombra sob o texto"),
    (
        "What keeps it readable over a bright window",
        "O que a mantém legível sobre uma janela clara",
    ),
    ("Darkness behind the line", "Escuridão atrás da linha"),
    (
        "Per cent. Zero shows nothing behind the words",
        "Em por cento. Zero não mostra nada atrás das palavras",
    ),
    ("Line just sung", "Linha recém-cantada"),
    (
        "Kept above the current one, dimmed",
        "Fica acima da atual, esmaecida",
    ),
    ("Lines still to come", "Próximas linhas"),
    (
        "Shown dimmed underneath. At least one is needed for the line to rise into place",
        "Aparecem esmaecidas embaixo. Ao menos uma é necessária para a linha subir até o lugar",
    ),
    ("Karaoke", "Karaokê"),
    (
        "Fills the line as the song moves through it",
        "Preenche a linha conforme a música avança",
    ),
    ("Hide while paused", "Esconder quando pausado"),
    (
        "Lyrics on screen with nothing playing is the most confusing thing it can do",
        "Letra na tela sem nada tocando é a coisa mais confusa que ele pode fazer",
    ),
    ("Position", "Posição"),
    (
        "A layer surface belongs to one screen and cannot be dragged to another.",
        "Uma superfície de camada pertence a uma tela e não pode ser arrastada para outra.",
    ),
    ("Let me move it", "Deixar arrastar"),
    (
        "The overlay takes your clicks while this is on, so you can drag it",
        "O overlay aceita seus cliques enquanto isto está ligado, para você arrastá-lo",
    ),
    ("Screen", "Tela"),
    (
        "Whichever the compositor picks",
        "A que o compositor escolher",
    ),
    ("Timing", "Sincronia"),
    (
        "Positive holds the lyrics back, negative brings them forward.",
        "Positivo atrasa a letra, negativo adianta.",
    ),
    ("Offset in milliseconds", "Ajuste em milissegundos"),
    ("Keys", "Teclas"),
    ("Show and hide", "Mostrar e esconder"),
    ("Move it", "Mover"),
    (
        "Line for your configuration",
        "Linha para a sua configuração",
    ),
    ("Copy", "Copiar"),
    ("Copied", "Copiado"),
    ("Starting", "Início"),
    (
        "The switch reads the file it writes, so it can never show on for something that is off.",
        "A chave lê o arquivo que ela escreve, então nunca mostra ligado o que está desligado.",
    ),
    ("Start with the session", "Iniciar com a sessão"),
    (
        "Opens when you log in, with the overlay ready",
        "Abre ao entrar na sessão, com o overlay pronto",
    ),
    ("Closing", "Fechar"),
    ("Quit LyricsLens", "Sair do LyricsLens"),
    (
        "Closes the overlay and leaves the status bar",
        "Fecha o overlay e sai da barra de status",
    ),
    ("A newer version is out", "Há uma versão mais nova"),
    (
        "Installs it over this one. Nothing happens until you run it",
        "Instala por cima desta. Nada acontece até você rodar",
    ),
    (
        "In pixels. Where a long line wraps",
        "Em pixels. Onde uma linha longa quebra",
    ),
    ("Looking…", "Procurando…"),
    ("lines being followed.", "linhas sendo acompanhadas."),
    (
        "No synced lyrics found for this one.",
        "Nenhuma letra sincronizada encontrada para esta.",
    ),
    // What the overlay says when there is nothing to sing.
    ("looking for the lyrics…", "procurando a letra…"),
    (
        "no synced lyrics for this one",
        "sem letra sincronizada para esta",
    ),
    ("waiting for the player", "esperando o player"),
    (
        "this player does not report its position",
        "este player não informa a posição",
    ),
    ("unknown track", "faixa desconhecida"),
    ("drag me", "arraste-me"),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// One test, because they all change the same environment variable and
    /// `cargo test` runs them in threads.
    #[test]
    fn what_is_translated_and_what_is_not() {
        unsafe { std::env::set_var("LC_ALL", "pt_BR.UTF-8") };
        assert_eq!(language(), Language::Portuguese);
        assert_eq!(t("Quit"), "Sair");
        assert_eq!(t("Nothing here says this"), "Nothing here says this");

        unsafe { std::env::set_var("LC_ALL", "fr_FR.UTF-8") };
        assert_eq!(language(), Language::English);
        assert_eq!(t("Quit"), "Quit");

        unsafe { std::env::remove_var("LC_ALL") };
    }
}
