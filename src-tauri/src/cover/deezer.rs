//! Capa pelo Deezer.
//!
//! A #3 lista iTunes primeiro, e foi o primeiro que eu testei. Medido antes de
//! escolher:
//!
//! | consulta | iTunes | Deezer |
//! |---|---|---|
//! | Radiohead — Pablo Honey | *boy pablo — Honey*, *The Doraemons* | acerta |
//! | Björk — Homogenic | *Homogenic (Live)* | acerta |
//! | Marisa Monte — Infinito Particular | acerta | acerta |
//!
//! O iTunes erra dois dos três, e erra de um jeito ruim: devolve **outro
//! artista** com confiança. O Deezer acerta os três e ainda entrega a capa em
//! várias resoluções direto na resposta, sem o truque de reescrever a URL que o
//! iTunes exige.
//!
//! Nenhum dos dois pede chave.

use serde::Deserialize;

use super::{parecidos, Cover, CoverError, CoverProvider};

const BASE: &str = "https://api.deezer.com";

#[derive(Debug, Deserialize)]
struct Busca {
    #[serde(default)]
    data: Vec<Album>,
}

#[derive(Debug, Deserialize)]
struct Album {
    #[serde(default)]
    title: String,
    #[serde(default)]
    artist: Artista,
    /// A maior que o Deezer publica (1000x1000). As menores existem, mas o
    /// overlay pode ser redimensionado e capa esticada fica borrada.
    #[serde(default)]
    cover_xl: Option<String>,
    #[serde(default)]
    cover_big: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Artista {
    #[serde(default)]
    name: String,
}

impl Album {
    fn capa(&self) -> Option<&str> {
        self.cover_xl
            .as_deref()
            .or(self.cover_big.as_deref())
            .filter(|u| !u.is_empty())
    }
}

pub struct Deezer {
    /// `None` pelo mesmo motivo do `LrcLib`: sem certificados não há cliente.
    http: Option<reqwest::Client>,
    base: String,
}

impl Deezer {
    pub fn new(http: Option<reqwest::Client>) -> Self {
        Self {
            http,
            base: BASE.into(),
        }
    }

    #[cfg(test)]
    fn com_base(http: reqwest::Client, base: impl Into<String>) -> Self {
        Self {
            http: Some(http),
            base: base.into(),
        }
    }
}

/// O álbum certo entre os resultados.
///
/// **Artista tem que bater.** Sem isso a busca por "Creep" devolve Vitamin
/// String Quartet e uma dúzia de covers — medido. Entre os do artista certo,
/// o título exato ganha; se nenhum bate exatamente, aceita o primeiro dele,
/// porque o serviço já ordena por relevância dentro do artista.
fn escolher<'a>(albuns: &'a [Album], artist: &str, album: &str) -> Option<&'a Album> {
    let do_artista: Vec<&Album> = albuns
        .iter()
        .filter(|a| parecidos(&a.artist.name, artist))
        .collect();

    do_artista
        .iter()
        .find(|a| parecidos(&a.title, album))
        .or(do_artista.first())
        .copied()
}

impl CoverProvider for Deezer {
    async fn find(&self, artist: &str, album: &str) -> Result<Option<Cover>, CoverError> {
        if artist.trim().is_empty() || album.trim().is_empty() {
            return Ok(None);
        }

        // Sem cliente não há capa — e isso não merece erro na tela, porque a
        // busca de letra já reclamou pelo mesmo motivo.
        let Some(http) = self.http.as_ref() else {
            return Ok(None);
        };

        let busca: Busca = http
            .get(format!("{}/search/album", self.base))
            .query(&[("q", format!("{artist} {album}")), ("limit", "10".into())])
            .send()
            .await
            .map_err(|e| CoverError::Network(e.to_string()))?
            .json()
            .await
            .map_err(|e| CoverError::Decode(e.to_string()))?;

        Ok(escolher(&busca.data, artist, album)
            .and_then(|a| a.capa())
            .map(|url| Cover {
                url: url.to_string(),
                source: "deezer".into(),
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn album(artista: &str, titulo: &str, capa: Option<&str>) -> Album {
        Album {
            title: titulo.into(),
            artist: Artista {
                name: artista.into(),
            },
            cover_xl: capa.map(String::from),
            cover_big: None,
        }
    }

    #[test]
    fn escolhe_o_album_do_artista_certo() {
        let achados = vec![
            album("Vitamin String Quartet", "Pablo Honey", Some("errada")),
            album("Radiohead", "Pablo Honey", Some("certa")),
        ];
        let e = escolher(&achados, "Radiohead", "Pablo Honey").unwrap();
        assert_eq!(e.capa(), Some("certa"));
    }

    /// Entre discos do artista certo, o título exato ganha — foi assim que o
    /// iTunes entregou *Homogenic (Live)* no lugar de *Homogenic*.
    #[test]
    fn o_titulo_exato_ganha_do_parecido() {
        let achados = vec![
            album("Björk", "Homogenic (Live)", Some("ao vivo")),
            album("Björk", "Homogenic", Some("estudio")),
        ];
        let e = escolher(&achados, "Björk", "Homogenic").unwrap();
        assert_eq!(e.capa(), Some("estudio"));
    }

    #[test]
    fn sem_o_artista_nos_resultados_nao_inventa() {
        let achados = vec![album("boy pablo", "Honey - Single", Some("errada"))];
        assert!(escolher(&achados, "Radiohead", "Pablo Honey").is_none());
    }

    /// Título diferente mas artista certo ainda serve: o player às vezes manda
    /// "Pablo Honey (Remastered)" e o disco continua sendo aquele.
    #[test]
    fn artista_certo_com_titulo_diferente_ainda_serve() {
        let achados = vec![album("Radiohead", "Pablo Honey (Remastered)", Some("ok"))];
        let e = escolher(&achados, "Radiohead", "Pablo Honey").unwrap();
        assert_eq!(e.capa(), Some("ok"));
    }

    #[test]
    fn album_sem_capa_publicada_nao_vira_url_vazia() {
        let achados = vec![album("Radiohead", "Pablo Honey", None)];
        assert!(escolher(&achados, "Radiohead", "Pablo Honey")
            .unwrap()
            .capa()
            .is_none());
    }

    #[tokio::test]
    async fn sem_artista_ou_album_nem_vai_a_rede() {
        // Base inalcançável de propósito: se ele tentar, o teste falha.
        let d = Deezer::com_base(reqwest::Client::new(), "http://127.0.0.1:1");
        assert_eq!(d.find("", "Pablo Honey").await.unwrap(), None);
        assert_eq!(d.find("Radiohead", "  ").await.unwrap(), None);
    }
}
