//! Detecção do que está tocando no sistema.
//!
//! O resto do app nunca fala com D-Bus, WinRT ou qualquer API de plataforma:
//! fala com [`MediaEvent`]. Trocar de plataforma é escrever um novo provider,
//! não mexer no app.

#[cfg(target_os = "linux")]
pub mod mpris;

use serde::Serialize;
use tokio::sync::mpsc;

#[cfg(target_os = "linux")]
pub use mpris::MprisProvider as PlatformProvider;

/// A faixa tocando agora, já normalizada — sem tipos de plataforma vazando.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<u64>,
    pub art_url: Option<String>,
    /// URL da mídia, quando existe. É o que dá para distinguir um vídeo de
    /// YouTube (título caótico) de uma faixa de streaming (título limpo).
    pub url: Option<String>,
    /// Quem está tocando: `firefox`, `spotify`, `vlc`…
    pub source: String,
}

impl Track {
    /// Chave estável da faixa, para cache e mapeamentos salvos pelo usuário.
    #[allow(dead_code, reason = "usada pelo cache de letras, ainda não escrito")]
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.artist.to_lowercase().trim(),
            self.title.to_lowercase().trim(),
            self.album.to_lowercase().trim()
        )
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.artist.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackState {
    Playing,
    Paused,
    #[default]
    Stopped,
}

impl PlaybackState {
    pub fn is_playing(self) -> bool {
        matches!(self, Self::Playing)
    }
}

/// Quanta confiança a âncora de posição merece.
///
/// Ver `docs/ARCHITECTURE.md`: fontes que quantizam a posição em 1s (Firefox)
/// só dão precisão real no instante em que o valor *muda*.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AnchorConfidence {
    /// A posição mudou neste instante — erro limitado ao intervalo de poll.
    Edge,
    /// Leitura avulsa. Pode carregar todo o erro de quantização da fonte.
    Sample,
}

#[derive(Clone, Debug, Serialize)]
// `rename_all` sozinho só renomeia as variantes; os campos dentro delas
// precisam de `rename_all_fields`, senão chegam em snake_case na UI.
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MediaEvent {
    TrackChanged {
        track: Track,
    },
    PlaybackChanged {
        state: PlaybackState,
    },
    /// A posição real da faixa em um instante conhecido.
    PositionAnchored {
        position_ms: u64,
        confidence: AnchorConfidence,
    },
    /// O usuário pulou na faixa — a estimativa anterior foi invalidada.
    Seeked {
        position_ms: u64,
    },
    /// Não há mais nenhum player no sistema.
    Gone,
}

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("falha no barramento: {0}")]
    Bus(String),
}

/// Contrato de qualquer detector de mídia.
///
/// A implementação roda até o canal fechar, empurrando eventos conforme
/// observa o sistema. Quem consome nunca faz polling.
pub trait MediaProvider: Send + 'static {
    fn run(
        self,
        tx: mpsc::Sender<MediaEvent>,
    ) -> impl std::future::Future<Output = Result<(), MediaError>> + Send;
}
