//! Capa do álbum.
//!
//! O MPRIS tem `mpris:artUrl` na especificação, mas nem todo player o preenche
//! — medido na fase 0: o Firefox entrega vazio. Quando o player dá, usamos o
//! dele; quando não dá, a capa vem da rede. Ver #3.
//!
//! A capa é do **álbum**, não da faixa: todas as músicas de um disco
//! compartilham a mesma, e é por isso que o cache é indexado por artista +
//! álbum. Um disco de doze faixas custa uma busca, não doze.

pub mod deezer;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoverError {
    #[error("falha de rede: {0}")]
    Network(String),
    #[error("resposta inválida: {0}")]
    Decode(String),
}

/// A capa encontrada e de onde veio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cover {
    pub url: String,
    /// `player`, `deezer` — aparece no log e ajuda a explicar capa errada.
    pub source: String,
}

/// Contrato de qualquer fonte de capa, no mesmo formato de `LyricsProvider`.
pub trait CoverProvider: Send + Sync {
    fn find(
        &self,
        artist: &str,
        album: &str,
    ) -> impl std::future::Future<Output = Result<Option<Cover>, CoverError>> + Send;
}

/// Chave de cache: artista + álbum, normalizados.
///
/// Caixa e espaço não distinguem discos, e o mesmo álbum chega escrito de
/// formas diferentes conforme o player.
pub fn chave_do_album(artist: &str, album: &str) -> String {
    format!(
        "{}|{}",
        artist.trim().to_lowercase(),
        album.trim().to_lowercase()
    )
}

/// Comparação frouxa de nomes, para casar o que o player diz com o que o
/// serviço devolve.
///
/// Só caixa, espaço e pontuação: nada de aproximação por distância. Um
/// "parecido o suficiente" aqui vira capa errada na tela, que é pior que capa
/// nenhuma — quem vê a capa de outro disco acha que o app se confundiu de
/// música, não de imagem.
pub fn parecidos(a: &str, b: &str) -> bool {
    let limpar = |s: &str| {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    limpar(a) == limpar(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chave_ignora_caixa_e_espaco() {
        assert_eq!(
            chave_do_album("  Radiohead ", "Pablo Honey"),
            chave_do_album("RADIOHEAD", "  pablo honey  ")
        );
    }

    #[test]
    fn a_chave_separa_discos_diferentes_do_mesmo_artista() {
        assert_ne!(
            chave_do_album("Radiohead", "Pablo Honey"),
            chave_do_album("Radiohead", "OK Computer")
        );
    }

    #[test]
    fn nomes_parecidos_ignoram_pontuacao_e_caixa() {
        assert!(parecidos("Björk", "björk"));
        assert!(parecidos("Sgt. Pepper's", "Sgt Peppers"));
        assert!(parecidos("A  Night   at the Opera", "A Night at the Opera"));
    }

    /// O que **não** pode casar: disco diferente é capa errada na tela.
    #[test]
    fn nomes_diferentes_nao_casam() {
        assert!(!parecidos("Pablo Honey", "OK Computer"));
        assert!(!parecidos("Homogenic", "Homogenic (Live)"));
        assert!(!parecidos("Creep", "Creep (Acoustic)"));
    }
}
