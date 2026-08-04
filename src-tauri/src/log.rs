//! Log em arquivo.
//!
//! O `stderr` não serve para app lançado pelo menu do sistema: ninguém o vê. O
//! usuário clica no ícone e "não acontece nada" — foi assim com a busca
//! estourando o tempo limite, com o compositor recusando atalho e com o cache
//! ilegível. Ver #14.
//!
//! O princípio da issue divide as duas metades: **erro que o usuário pode
//! resolver aparece na tela** (por `UiError`, num evento que a janela de
//! configurações escuta); **erro que ele não pode resolver fica aqui**, para
//! quem for investigar depois.
//!
//! Escrever no `stderr` continua acontecendo. Quem roda `pnpm tauri dev` quer
//! ver a linha no terminal, e quem roda o app instalado quer ela no arquivo —
//! não é ou-um-ou-outro.

use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// A partir daqui o arquivo é rotacionado.
///
/// Um mega dá semanas de uso normal e ainda abre instantâneo num editor. O
/// arquivo anterior é preservado como `.1`, então o teto de disco é 2 MiB —
/// log sem teto é um jeito lento de encher a partição de quem só queria ver
/// letra de música.
const LIMITE_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nivel {
    Info,
    Aviso,
    Erro,
}

impl fmt::Display for Nivel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Nivel::Info => "info",
            Nivel::Aviso => "aviso",
            Nivel::Erro => "erro",
        })
    }
}

/// `~/.local/state/lyricslens`, respeitando `XDG_STATE_HOME`.
///
/// Estado é o lugar certo pela especificação XDG: não é configuração (o
/// usuário não edita), não é cache (não pode ser apagado a esmo sem perder
/// histórico), e não é dado essencial.
pub fn diretorio() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_STATE_HOME") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => PathBuf::from(std::env::var_os("HOME")?)
            .join(".local")
            .join("state"),
    };
    Some(base.join("lyricslens"))
}

pub fn arquivo() -> Option<PathBuf> {
    Some(diretorio()?.join("lyricslens.log"))
}

fn anterior() -> Option<PathBuf> {
    Some(diretorio()?.join("lyricslens.log.1"))
}

fn destino() -> &'static Mutex<Option<std::fs::File>> {
    static DESTINO: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    DESTINO.get_or_init(|| Mutex::new(abrir()))
}

/// Rotaciona se precisar e abre para anexar.
///
/// Devolve `None` quando não dá para escrever — disco cheio, `HOME` ausente,
/// permissão negada. Log é diagnóstico: se ele próprio falhar, o app segue.
fn abrir() -> Option<std::fs::File> {
    let alvo = arquivo()?;
    std::fs::create_dir_all(alvo.parent()?).ok()?;

    if std::fs::metadata(&alvo).map(|m| m.len()).unwrap_or(0) >= LIMITE_BYTES {
        // `rename` por cima do `.1` antigo: dois arquivos, teto conhecido.
        let _ = std::fs::rename(&alvo, anterior()?);
    }

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&alvo)
        .ok()
}

/// Uma linha no log e no `stderr`.
pub fn escrever(nivel: Nivel, alvo: &str, mensagem: &str) {
    let linha = format!("{} [{nivel}] {alvo}: {mensagem}", agora_iso());
    eprintln!("{linha}");

    if let Ok(mut guarda) = destino().lock() {
        if let Some(arquivo) = guarda.as_mut() {
            // Falha ao escrever não pode derrubar o app nem virar recursão.
            let _ = writeln!(arquivo, "{linha}");
        }
    }
}

/// Uma linha no log, no formato `alvo: mensagem`.
///
/// `logar!(erro, "hotkey", "o compositor recusou {atalho}")`
macro_rules! logar {
    ($nivel:ident, $alvo:expr, $($arg:tt)*) => {
        $crate::log::escrever(
            $crate::log::Nivel::$nivel,
            $alvo,
            &format!($($arg)*),
        )
    };
}
pub(crate) use logar;

// ---------------------------------------------------------------------------
// Carimbo de tempo
//
// Sem crate de data: são vinte linhas de aritmética contra uma dependência
// inteira, e o log é o único lugar do app que precisa disso. UTC de propósito
// — fuso exigiria ler a base de zoneinfo, e log com fuso implícito é pior que
// log em UTC declarado, porque quem lê não sabe qual era.
// ---------------------------------------------------------------------------

fn agora_iso() -> String {
    let segundos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    iso_de_epoch(segundos)
}

/// Segundos desde o epoch → `2026-08-04T17:45:12Z`.
fn iso_de_epoch(segundos: i64) -> String {
    let dias = segundos.div_euclid(86_400);
    let resto = segundos.rem_euclid(86_400);
    let (ano, mes, dia) = civil_de_dias(dias);
    format!(
        "{ano:04}-{mes:02}-{dia:02}T{:02}:{:02}:{:02}Z",
        resto / 3600,
        (resto % 3600) / 60,
        resto % 60
    )
}

/// Dias desde 1970-01-01 → data civil.
///
/// Algoritmo do Howard Hinnant: desloca o início do ano para março, o que faz
/// o dia bissexto cair no fim e some com o caso especial de fevereiro.
fn civil_de_dias(dias: i64) -> (i64, u32, u32) {
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // dia da era, 0..=146096
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // ano da era
    let ano = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // dia do ano, contado de março
    let mp = (5 * doy + 2) / 153; // mês deslocado, 0=março
    let dia = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if mes <= 2 { ano + 1 } else { ano }, mes, dia)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_e_a_data_de_referencia() {
        assert_eq!(iso_de_epoch(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn converte_um_instante_conhecido() {
        // 2026-08-04T17:45:12Z, conferido contra `date -u -d @1785865512`.
        assert_eq!(iso_de_epoch(1_785_865_512), "2026-08-04T17:45:12Z");
    }

    /// O 29 de fevereiro é onde toda aritmética de data caseira quebra.
    #[test]
    fn acerta_o_dia_bissexto() {
        // 2024-02-29T12:00:00Z
        assert_eq!(iso_de_epoch(1_709_208_000), "2024-02-29T12:00:00Z");
        // 2000-02-29: ano divisível por 100 *e* por 400, o caso que a regra
        // simplificada de bissexto erra.
        assert_eq!(iso_de_epoch(951_825_600), "2000-02-29T12:00:00Z");
        // 1900 não foi bissexto — 1900-03-01, não 1900-02-29.
        assert_eq!(iso_de_epoch(-2_203_891_200), "1900-03-01T00:00:00Z");
    }

    #[test]
    fn viradas_de_ano_e_de_dia() {
        assert_eq!(iso_de_epoch(1_767_225_599), "2025-12-31T23:59:59Z");
        assert_eq!(iso_de_epoch(1_767_225_600), "2026-01-01T00:00:00Z");
    }

    /// Data anterior ao epoch não pode virar lixo: o relógio do sistema pode
    /// estar errado, e um log com data absurda é melhor que um log que entra
    /// em pânico.
    #[test]
    fn aguenta_tempo_negativo() {
        assert_eq!(iso_de_epoch(-1), "1969-12-31T23:59:59Z");
        assert_eq!(iso_de_epoch(-86_400), "1969-12-31T00:00:00Z");
    }

    #[test]
    fn o_diretorio_respeita_o_xdg_state_home() {
        // Sem mexer no ambiente do processo de teste: a regra é simples o
        // bastante para ser conferida pela forma do caminho.
        let dir = diretorio().expect("HOME existe no ambiente de teste");
        assert!(dir.ends_with("lyricslens"), "terminou em {dir:?}");
    }
}
