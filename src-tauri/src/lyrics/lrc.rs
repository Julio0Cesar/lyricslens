//! Parser do formato LRC.
//!
//! O formato parece trivial e não é. Os três detalhes que quase toda
//! implementação erra:
//!
//! 1. **Uma linha pode ter vários timestamps** — `[00:12][01:30]refrão` é LRC
//!    válido e significa que o refrão toca duas vezes.
//! 2. **`[offset:+500]`** é ajuste de sincronia de graça, embutido no arquivo.
//! 3. **Linha sem texto é conteúdo, não lixo** — é ela que apaga a tela no
//!    intervalo instrumental. Descartar deixa a última frase congelada na
//!    cara do usuário por 30 segundos.

use super::LyricLine;

/// Resultado do parse, já ordenado por tempo.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ParsedLrc {
    pub lines: Vec<LyricLine>,
    pub offset_ms: i64,
}

/// Extrai um timestamp `[mm:ss.xx]` a partir de `pos`, se houver.
/// Devolve o tempo em ms e onde o colchete fechou.
fn parse_timestamp(s: &str, pos: usize) -> Option<(i64, usize)> {
    let rest = s.get(pos..)?;
    if !rest.starts_with('[') {
        return None;
    }
    let fim = rest.find(']')?;
    let corpo = &rest[1..fim];

    let (min, resto) = corpo.split_once(':')?;
    let min: i64 = min.trim().parse().ok()?;

    // Os centésimos podem vir com 2 ou 3 casas, ou não vir.
    let seg: f64 = resto.trim().replace(',', ".").parse().ok()?;
    if !seg.is_finite() || seg < 0.0 {
        return None;
    }

    Some((min * 60_000 + (seg * 1000.0).round() as i64, pos + fim + 1))
}

/// Tags de metadado: `[ar:…]`, `[offset:+500]`. Devolve (chave, valor).
fn parse_tag(s: &str) -> Option<(&str, &str)> {
    let corpo = s.strip_prefix('[')?.strip_suffix(']')?;
    let (k, v) = corpo.split_once(':')?;
    let k = k.trim();
    // Um timestamp também casa com "algo:algo" — o que distingue é a chave
    // ser alfabética.
    k.chars()
        .all(|c| c.is_ascii_alphabetic())
        .then(|| (k, v.trim()))
}

pub fn parse(lrc: &str) -> ParsedLrc {
    let mut lines: Vec<LyricLine> = Vec::new();
    let mut offset_ms: i64 = 0;

    for raw in lrc.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }

        if let Some((k, v)) = parse_tag(raw) {
            if k.eq_ignore_ascii_case("offset") {
                if let Ok(n) = v.replace('+', "").trim().parse::<i64>() {
                    offset_ms = n;
                }
            }
            continue;
        }

        // Todos os timestamps que abrem a linha.
        let mut tempos = Vec::new();
        let mut pos = 0usize;
        while let Some((ms, prox)) = parse_timestamp(raw, pos) {
            tempos.push(ms);
            pos = prox;
        }
        if tempos.is_empty() {
            continue;
        }

        let texto = raw[pos..].trim().to_string();
        for ms in tempos {
            lines.push(LyricLine {
                at_ms: ms,
                text: texto.clone(),
            });
        }
    }

    // `[offset:+500]` adianta a letra: por convenção do formato, o valor é
    // subtraído do timestamp.
    if offset_ms != 0 {
        for l in &mut lines {
            l.at_ms -= offset_ms;
        }
    }

    lines.sort_by_key(|l| l.at_ms);
    lines.dedup_by(|a, b| a.at_ms == b.at_ms && a.text == b.text);

    ParsedLrc { lines, offset_ms }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_timestamp_com_e_sem_centesimos() {
        let p = parse("[00:04.04] primeira\n[01:10] segunda\n[02:03.500] terceira");
        assert_eq!(p.lines.len(), 3);
        assert_eq!(p.lines[0].at_ms, 4_040);
        assert_eq!(p.lines[1].at_ms, 70_000);
        assert_eq!(p.lines[2].at_ms, 123_500);
    }

    #[test]
    fn varios_timestamps_na_mesma_linha_viram_varias_ocorrencias() {
        let p = parse("[00:12.00][01:30.00]refrão");
        assert_eq!(p.lines.len(), 2, "o refrão toca duas vezes");
        assert_eq!(p.lines[0].at_ms, 12_000);
        assert_eq!(p.lines[1].at_ms, 90_000);
        assert_eq!(p.lines[0].text, "refrão");
        assert_eq!(p.lines[1].text, "refrão");
    }

    #[test]
    fn linha_vazia_com_timestamp_e_preservada() {
        // É ela que limpa a tela no instrumental.
        let p = parse("[00:10.00]cantando\n[00:20.00]\n[00:40.00]voltou");
        assert_eq!(p.lines.len(), 3);
        assert_eq!(p.lines[1].at_ms, 20_000);
        assert_eq!(p.lines[1].text, "");
    }

    #[test]
    fn offset_adianta_a_letra() {
        let p = parse("[offset:+500]\n[00:10.00]linha");
        assert_eq!(p.offset_ms, 500);
        assert_eq!(p.lines[0].at_ms, 9_500);

        let p = parse("[offset:-250]\n[00:10.00]linha");
        assert_eq!(p.lines[0].at_ms, 10_250);
    }

    #[test]
    fn tags_de_metadado_nao_viram_linha() {
        let p = parse("[ar:Marisa Monte]\n[ti:Infinito Particular]\n[00:04.04]eis o melhor");
        assert_eq!(p.lines.len(), 1);
        assert_eq!(p.lines[0].text, "eis o melhor");
    }

    #[test]
    fn ordena_e_remove_duplicata_exata() {
        let p = parse("[00:30.00]b\n[00:10.00]a\n[00:30.00]b");
        assert_eq!(p.lines.len(), 2);
        assert_eq!(p.lines[0].text, "a");
        assert_eq!(p.lines[1].text, "b");
    }

    #[test]
    fn lixo_sem_timestamp_e_ignorado() {
        let p = parse("isso não é LRC\n\n[00:10.00]isso é");
        assert_eq!(p.lines.len(), 1);
    }


}
