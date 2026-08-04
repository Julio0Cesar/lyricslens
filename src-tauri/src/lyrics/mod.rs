//! Busca e representação de letras.

pub mod lrc;
pub mod lrclib;
pub mod normalize;

use serde::{Deserialize, Serialize};

use crate::media::Track;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub at_ms: i64,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    /// Vazio quando só existe letra sem sincronia.
    pub lines: Vec<LyricLine>,
    /// Letra corrida, quando não há versão sincronizada.
    pub plain: Option<String>,
    pub instrumental: bool,
    /// De onde veio: `lrclib`, `cache`…
    pub source: String,
    /// Id no provedor, para reencontrar a mesma letra depois.
    pub provider_id: Option<String>,
    /// Reservado para a tradução (issue #1). Existir desde já evita migrar
    /// o cache depois.
    pub translation: Option<Vec<LyricLine>>,
}

impl Lyrics {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.plain.is_none() && !self.instrumental
    }
}

/// Uma opção quando a busca exata falha e é o usuário quem decide.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub provider_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_s: Option<f64>,
    pub has_synced: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum LyricsError {
    #[error("letra não encontrada")]
    NotFound,
    #[error("falha de rede: {0}")]
    Network(String),
    #[error("resposta inválida: {0}")]
    Decode(String),
}

/// Como cada falha de busca chega à tela.
///
/// O `Decode` vira um código próprio de propósito: "o LRCLIB respondeu algo
/// estranho" e "não consegui falar com o LRCLIB" mandam o usuário para lados
/// diferentes — um é problema do serviço, o outro pode ser a rede dele.
impl From<LyricsError> for crate::i18n::UiError {
    fn from(e: LyricsError) -> Self {
        use crate::i18n::UiError;
        match e {
            LyricsError::NotFound => UiError::new("lyrics.notFound"),
            LyricsError::Network(m) => UiError::new("lyrics.network").arg("motivo", m),
            LyricsError::Decode(m) => UiError::new("lyrics.decode").arg("motivo", m),
        }
    }
}

/// Contrato de qualquer fonte de letra.
pub trait LyricsProvider: Send + Sync {
    /// Busca exata pela assinatura da faixa.
    fn fetch(
        &self,
        track: &Track,
    ) -> impl std::future::Future<Output = Result<Lyrics, LyricsError>> + Send;

    /// Alternativas para o usuário escolher quando a busca exata falha.
    fn search(
        &self,
        artist: &str,
        title: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Candidate>, LyricsError>> + Send;

    /// Busca uma letra já identificada — usada pelo fallback de busca ampla e,
    /// mais tarde, pelo mapeamento manual salvo pelo usuário.
    fn fetch_by_id(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Lyrics, LyricsError>> + Send;
}
