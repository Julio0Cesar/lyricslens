//! Cliente do LRCLIB — <https://lrclib.net>.
//!
//! Grátis, sem autenticação, com letra sincronizada. A busca exata (`/api/get`)
//! casa por assinatura da faixa e tolera pequena diferença de duração; quando
//! ela falha, `/api/search` devolve candidatos para o usuário escolher.

use serde::Deserialize;

use super::{lrc, Candidate, Lyrics, LyricsError, LyricsProvider};
use crate::media::Track;

const BASE: &str = "https://lrclib.net/api";
/// O LRCLIB pede identificação honesta do cliente.
const USER_AGENT: &str = concat!(
    "LyricsLens/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/Julio0Cesar/lyricslens)"
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrcLibTrack {
    id: i64,
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    album_name: String,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    plain_lyrics: Option<String>,
    #[serde(default)]
    synced_lyrics: Option<String>,
}

impl LrcLibTrack {
    fn into_lyrics(self) -> Lyrics {
        let lines = self
            .synced_lyrics
            .as_deref()
            .map(|s| lrc::parse(s).lines)
            .unwrap_or_default();

        Lyrics {
            lines,
            plain: self.plain_lyrics.filter(|s| !s.trim().is_empty()),
            instrumental: self.instrumental,
            source: "lrclib".into(),
            provider_id: Some(self.id.to_string()),
            translation: None,
        }
    }

    fn into_candidate(self) -> Candidate {
        Candidate {
            provider_id: self.id.to_string(),
            title: self.track_name,
            artist: self.artist_name,
            album: self.album_name,
            duration_s: self.duration,
            has_synced: self.synced_lyrics.is_some_and(|s| !s.trim().is_empty()),
        }
    }
}

pub struct LrcLib {
    http: reqwest::Client,
}

impl LrcLib {
    pub fn new() -> Result<Self, LyricsError> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| LyricsError::Network(e.to_string()))?;
        Ok(Self { http })
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, String)],
    ) -> Result<T, LyricsError> {
        let resp = self
            .http
            .get(url)
            .query(params)
            .send()
            .await
            .map_err(|e| LyricsError::Network(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(LyricsError::NotFound);
        }
        if !resp.status().is_success() {
            return Err(LyricsError::Network(format!("HTTP {}", resp.status())));
        }

        resp.json::<T>()
            .await
            .map_err(|e| LyricsError::Decode(e.to_string()))
    }
}

/// Diferença de duração que ainda aceitamos ao escolher um candidato.
/// Acima disto é outra gravação — ao vivo, estendida, remix — e a letra
/// sincronizada não serviria de nada.
const TOLERANCIA_S: f64 = 12.0;

/// A melhor alternativa entre os candidatos da busca ampla.
///
/// Letra sincronizada ganha de letra corrida sempre; entre as sincronizadas,
/// vence a duração mais próxima.
fn escolher(candidatos: &[Candidate], duration_ms: Option<u64>) -> Option<&Candidate> {
    let alvo_s = duration_ms.map(|ms| ms as f64 / 1000.0);

    let dentro_da_tolerância = |c: &Candidate| match (alvo_s, c.duration_s) {
        (Some(alvo), Some(d)) => (d - alvo).abs() <= TOLERANCIA_S,
        // Sem duração para comparar, não dá para descartar por duração.
        _ => true,
    };

    let distância = |c: &Candidate| match (alvo_s, c.duration_s) {
        (Some(alvo), Some(d)) => (d - alvo).abs(),
        _ => f64::MAX,
    };

    candidatos
        .iter()
        .filter(|c| c.has_synced && dentro_da_tolerância(c))
        .min_by(|a, b| distância(a).total_cmp(&distância(b)))
}

impl LrcLib {
    /// Busca por assinatura exata. É a que acerta quando o metadado é limpo.
    async fn fetch_exact(&self, track: &Track) -> Result<Lyrics, LyricsError> {
        let q = super::normalize::preparar(&track.artist, &track.title);

        let mut params = vec![
            ("artist_name", q.artist),
            ("track_name", q.title),
            ("album_name", track.album.clone()),
        ];
        // A duração é o que desempata entre versões da mesma música.
        if let Some(ms) = track.duration_ms {
            params.push(("duration", (ms / 1000).to_string()));
        }

        let found: LrcLibTrack = self.get_json(&format!("{BASE}/get"), &params).await?;
        let lyrics = found.into_lyrics();

        if lyrics.is_empty() {
            return Err(LyricsError::NotFound);
        }
        Ok(lyrics)
    }
}

impl LyricsProvider for LrcLib {
    /// Exata primeiro; se falhar, busca ampla e escolha automática.
    ///
    /// O `/api/get` casa por assinatura completa: basta o álbum vir escrito
    /// diferente, ou a duração destoar, para virar 404 mesmo existindo a letra.
    /// A busca ampla recupera exatamente esses casos.
    async fn fetch(&self, track: &Track) -> Result<Lyrics, LyricsError> {
        match self.fetch_exact(track).await {
            Ok(lyrics) => return Ok(lyrics),
            Err(LyricsError::NotFound) => {}
            // Falha de rede não é ausência de letra — não vale insistir.
            Err(e) => return Err(e),
        }

        let candidatos = self.search(&track.artist, &track.title).await?;
        let escolhido = escolher(&candidatos, track.duration_ms).ok_or(LyricsError::NotFound)?;
        self.fetch_by_id(&escolhido.provider_id).await
    }

    async fn search(&self, artist: &str, title: &str) -> Result<Vec<Candidate>, LyricsError> {
        let q = super::normalize::preparar(artist, title);
        let params = vec![("artist_name", q.artist), ("track_name", q.title)];

        let achados: Vec<LrcLibTrack> = self.get_json(&format!("{BASE}/search"), &params).await?;
        Ok(achados.into_iter().map(|t| t.into_candidate()).collect())
    }

    async fn fetch_by_id(&self, id: &str) -> Result<Lyrics, LyricsError> {
        let found: LrcLibTrack = self.get_json(&format!("{BASE}/get/{id}"), &[]).await?;
        let lyrics = found.into_lyrics();
        if lyrics.is_empty() {
            return Err(LyricsError::NotFound);
        }
        Ok(lyrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faixa(synced: Option<&str>, plain: Option<&str>, instrumental: bool) -> LrcLibTrack {
        LrcLibTrack {
            id: 707294,
            track_name: "Infinito particular".into(),
            artist_name: "Marisa Monte".into(),
            album_name: "Infinito Particular".into(),
            duration: Some(252.0),
            instrumental,
            plain_lyrics: plain.map(str::to_string),
            synced_lyrics: synced.map(str::to_string),
        }
    }

    #[test]
    fn letra_sincronizada_vira_linhas() {
        let l = faixa(
            Some("[00:04.04] eis o melhor\n[00:10.55] no meu termômetro"),
            None,
            false,
        )
        .into_lyrics();
        assert!(!l.lines.is_empty());
        assert_eq!(l.lines.len(), 2);
        assert_eq!(l.lines[0].at_ms, 4_040);
        assert_eq!(l.provider_id.as_deref(), Some("707294"));
    }

    #[test]
    fn sem_sincronia_ainda_serve_como_texto() {
        let l = faixa(None, Some("eis o melhor e o pior de mim"), false).into_lyrics();
        assert!(l.lines.is_empty());
        assert!(!l.is_empty(), "letra corrida ainda é letra");
    }

    #[test]
    fn resposta_sem_nada_util_conta_como_vazia() {
        let l = faixa(None, None, false).into_lyrics();
        assert!(l.is_empty());
    }

    #[test]
    fn instrumental_nao_e_vazio() {
        // Saber que é instrumental é resposta: o overlay para de procurar.
        let l = faixa(None, None, true).into_lyrics();
        assert!(!l.is_empty());
        assert!(l.instrumental);
    }

    #[test]
    fn plain_so_com_espaco_e_descartado() {
        let l = faixa(None, Some("   \n  "), false).into_lyrics();
        assert!(l.is_empty());
    }

    fn cand(id: &str, dur: Option<f64>, synced: bool) -> Candidate {
        Candidate {
            provider_id: id.into(),
            title: "1 Thing".into(),
            artist: "Amerie".into(),
            album: "Touch".into(),
            duration_s: dur,
            has_synced: synced,
        }
    }

    #[test]
    fn escolhe_a_duracao_mais_proxima() {
        let cs = [
            cand("longe", Some(179.0), true),
            cand("perto", Some(236.0), true),
            cand("medio", Some(232.0), true),
        ];
        let e = escolher(&cs, Some(237_000)).expect("devia escolher");
        assert_eq!(e.provider_id, "perto");
    }

    #[test]
    fn ignora_candidato_sem_sincronia() {
        let cs = [
            cand("exato_sem_sync", Some(237.0), false),
            cand("distante_com_sync", Some(232.0), true),
        ];
        let e = escolher(&cs, Some(237_000)).unwrap();
        assert_eq!(
            e.provider_id, "distante_com_sync",
            "letra sincronizada vale mais que duração exata"
        );
    }

    #[test]
    fn recusa_gravacao_de_duracao_incompativel() {
        // Versão ao vivo de 6 minutos não serve para a de rádio de 4.
        let cs = [cand("ao_vivo", Some(360.0), true)];
        assert!(escolher(&cs, Some(237_000)).is_none());
    }

    #[test]
    fn sem_duracao_conhecida_aceita_o_primeiro_sincronizado() {
        let cs = [cand("a", None, false), cand("b", Some(300.0), true)];
        let e = escolher(&cs, None).unwrap();
        assert_eq!(e.provider_id, "b");
    }

    #[test]
    fn lista_vazia_nao_escolhe_nada() {
        assert!(escolher(&[], Some(237_000)).is_none());
    }
}
