//! Limpeza de título e artista antes da busca.
//!
//! Streaming entrega metadado limpo. YouTube entrega o que o uploader digitou:
//! `Musica | HD Ao Vivo`, `[Official Music Video]`, `(Lyrics)`, o canal no lugar
//! do artista. Sem isto a busca exata erra em quase todo vídeo.

/// Ruído que aparece dentro de parênteses ou colchetes e não faz parte do nome.
const RUIDO: &[&str] = &[
    "official video",
    "official music video",
    "official audio",
    "official lyric video",
    "official visualizer",
    "music video",
    "lyric video",
    "lyrics",
    "letra",
    "legendado",
    "tradução",
    "audio",
    "áudio",
    "hd",
    "hq",
    "4k",
    "8k",
    "1080p",
    "720p",
    "full hd",
    "clipe oficial",
    "video oficial",
    "vídeo oficial",
    "visualizer",
    "explicit",
    "clean",
    "free download",
];

/// Sufixos separados por barra vertical que são só decoração: `Musica | HD`.
fn tira_sufixo_barra(s: &str) -> String {
    match s.split_once('|') {
        Some((antes, depois)) if é_ruído(depois) => antes.trim().to_string(),
        _ => s.to_string(),
    }
}

fn é_ruído(texto: &str) -> bool {
    let t = texto.trim().to_lowercase();
    if t.is_empty() {
        return true;
    }
    RUIDO.iter().any(|r| t == *r || t.starts_with(&format!("{r} ")) || t.contains(r))
}

/// Remove blocos entre `()` ou `[]` cujo conteúdo é ruído conhecido.
/// Blocos com conteúdo útil — `(Remastered 2011)`, `(Acoustic)` — ficam:
/// eles ajudam a casar a versão certa.
fn tira_blocos(s: &str) -> String {
    let mut saída = String::with_capacity(s.len());
    let mut buffer = String::new();
    let mut fechamento: Option<char> = None;

    for c in s.chars() {
        match fechamento {
            None => match c {
                '(' => fechamento = Some(')'),
                '[' => fechamento = Some(']'),
                _ => saída.push(c),
            },
            Some(fim) if c == fim => {
                if !é_ruído(&buffer) {
                    saída.push(if fim == ')' { '(' } else { '[' });
                    saída.push_str(&buffer);
                    saída.push(fim);
                }
                buffer.clear();
                fechamento = None;
            }
            Some(_) => buffer.push(c),
        }
    }

    // Bloco aberto e nunca fechado: devolve como texto, não engole.
    if fechamento.is_some() && !buffer.is_empty() {
        saída.push_str(&buffer);
    }
    saída
}

/// `feat.`/`ft.` no título atrapalha a busca — o artista convidado raramente
/// está no registro da letra.
fn tira_participacao(s: &str) -> String {
    let baixo = s.to_lowercase();
    for marca in [" feat.", " feat ", " ft.", " ft ", " featuring "] {
        if let Some(i) = baixo.find(marca) {
            return s[..i].to_string();
        }
    }
    s.to_string()
}

fn colapsa_espaços(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn limpa(s: &str) -> String {
    let s = tira_sufixo_barra(s);
    let s = tira_blocos(&s);
    let s = tira_participacao(&s);
    colapsa_espaços(s.trim().trim_matches('-').trim())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consulta {
    pub artist: String,
    pub title: String,
}

/// Constrói a consulta a partir do que o player informou.
///
/// Quando o título traz `Artista - Música` e o artista informado é o canal
/// (`... - Topic`, `VEVO`), o que está no título vale mais.
pub fn preparar(artist: &str, title: &str) -> Consulta {
    let artista_limpo = limpa(&desnudar_canal(artist));
    let titulo_limpo = limpa(title);

    if let Some((a, t)) = separar_hifen(&titulo_limpo) {
        if artista_limpo.is_empty() || parece_canal(artist) {
            return Consulta { artist: a, title: t };
        }
    }

    Consulta {
        artist: artista_limpo,
        title: titulo_limpo,
    }
}

/// `Marisa Monte - Topic` → `Marisa Monte`.
fn desnudar_canal(artist: &str) -> String {
    let a = artist.trim();
    for sufixo in [" - Topic", "VEVO", " Official", "Official"] {
        if let Some(base) = a.strip_suffix(sufixo) {
            return base.trim().to_string();
        }
    }
    a.to_string()
}

fn parece_canal(artist: &str) -> bool {
    let a = artist.to_lowercase();
    a.ends_with("- topic") || a.contains("vevo") || a.contains("records") || a.contains("channel")
}

/// `Artista - Música` → `("Artista", "Música")`. Só o primeiro hífen conta.
fn separar_hifen(titulo: &str) -> Option<(String, String)> {
    let (a, t) = titulo.split_once(" - ")?;
    let (a, t) = (a.trim(), t.trim());
    (!a.is_empty() && !t.is_empty()).then(|| (a.to_string(), t.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tira_o_lixo_classico_do_youtube() {
        let c = preparar("", "Numb [Official Music Video] | HD");
        assert_eq!(c.title, "Numb");
    }

    #[test]
    fn preserva_informacao_de_versao() {
        // Isto ajuda a casar a gravação certa — não é ruído.
        let c = preparar("Linkin Park", "Numb (Remastered 2011)");
        assert_eq!(c.title, "Numb (Remastered 2011)");
    }

    #[test]
    fn separa_artista_do_titulo_quando_o_canal_e_generico() {
        let c = preparar("LinkinParkVEVO", "Linkin Park - Numb (Official Video)");
        assert_eq!(c.artist, "Linkin Park");
        assert_eq!(c.title, "Numb");
    }

    #[test]
    fn artista_de_verdade_ganha_do_hifen_no_titulo() {
        let c = preparar("Marisa Monte", "Infinito Particular");
        assert_eq!(c.artist, "Marisa Monte");
        assert_eq!(c.title, "Infinito Particular");
    }

    #[test]
    fn remove_o_sufixo_topic_do_youtube_music() {
        let c = preparar("Marisa Monte - Topic", "Infinito Particular");
        assert_eq!(c.artist, "Marisa Monte");
    }

    #[test]
    fn corta_participacao() {
        let c = preparar("Jay-Z", "Numb / Encore feat. Linkin Park");
        assert_eq!(c.title, "Numb / Encore");
    }

    #[test]
    fn nao_engole_parenteses_sem_fechamento() {
        let c = preparar("", "Música (ao vivo");
        assert_eq!(c.title, "Música ao vivo");
    }

    #[test]
    fn metadado_ja_limpo_passa_intacto() {
        let c = preparar("Michael Jackson", "Butterflies");
        assert_eq!(c.artist, "Michael Jackson");
        assert_eq!(c.title, "Butterflies");
    }
}
