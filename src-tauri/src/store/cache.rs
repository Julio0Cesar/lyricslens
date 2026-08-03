//! Cache local de letras.
//!
//! Serve a três propósitos que parecem um só e não são:
//!
//! 1. **Velocidade** — a mesma música tocada de novo não vai à rede.
//! 2. **Offline** — letra marcada como fixada fica disponível sem internet.
//! 3. **Educação com a API** — uma faixa sem letra é registrada como tal, para
//!    não martelar o LRCLIB a cada vez que ela tocar.

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

use crate::lyrics::{LyricLine, Lyrics};
use crate::media::Track;

/// Quanto tempo uma busca fracassada continua valendo antes de tentar de novo.
/// Letra nova é adicionada ao LRCLIB toda hora, então isto não pode ser eterno.
const MISS_TTL_SECS: i64 = 60 * 60 * 24 * 3;

pub struct Cache {
    conn: Mutex<Connection>,
}

/// Uma letra que o usuário mandou manter offline.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedLyrics {
    pub track_key: String,
    pub artist: String,
    pub title: String,
    /// Sincronizada palavra a palavra, ou só o texto.
    pub synced: bool,
}

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    /// Faixas com alguma letra guardada.
    pub tracks: i64,
    /// Destas, quantas têm letra sincronizada.
    pub synced: i64,
    /// Fixadas para uso offline.
    pub pinned: i64,
    /// Faixas conhecidas como sem letra — a busca nem é refeita.
    pub misses: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("banco: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("serialização: {0}")]
    Json(#[from] serde_json::Error),
}

fn agora() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Cache {
    pub fn open(path: &Path) -> Result<Self, CacheError> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self, CacheError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self, CacheError> {
        // WAL para não travar leitura durante escrita.
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS lyrics (
                track_key        TEXT PRIMARY KEY,
                artist           TEXT NOT NULL,
                title            TEXT NOT NULL,
                album            TEXT NOT NULL,
                duration_ms      INTEGER,
                source           TEXT NOT NULL,
                provider_id      TEXT,
                instrumental     INTEGER NOT NULL DEFAULT 0,
                lines_json       TEXT,
                plain            TEXT,
                -- Reservados para a tradução (issue #1). Existirem desde já
                -- evita migração de schema quando ela chegar.
                translation_json TEXT,
                translation_lang TEXT,
                -- Fixada pelo usuário: nunca expira, é o modo offline.
                pinned           INTEGER NOT NULL DEFAULT 0,
                fetched_at       INTEGER NOT NULL
            );


            -- Faixas para as quais não existe letra, para não repetir a busca.
            CREATE TABLE IF NOT EXISTS misses (
                track_key TEXT PRIMARY KEY,
                tried_at  INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get(&self, track_key: &str) -> Result<Option<Lyrics>, CacheError> {
        let conn = self.conn.lock().unwrap();
        let linha = conn
            .query_row(
                "SELECT lines_json, plain, instrumental, source, provider_id, translation_json
                 FROM lyrics WHERE track_key = ?1",
                params![track_key],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)? != 0,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()?;

        let Some((lines_json, plain, instrumental, source, provider_id, translation_json)) = linha
        else {
            return Ok(None);
        };

        let lines: Vec<LyricLine> = match lines_json {
            Some(j) => serde_json::from_str(&j)?,
            None => Vec::new(),
        };
        let translation: Option<Vec<LyricLine>> = match translation_json {
            Some(j) => Some(serde_json::from_str(&j)?),
            None => None,
        };

        Ok(Some(Lyrics {
            lines,
            plain,
            instrumental,
            source,
            provider_id,
            translation,
        }))
    }

    pub fn put(&self, track: &Track, lyrics: &Lyrics) -> Result<(), CacheError> {
        let lines_json = (!lyrics.lines.is_empty())
            .then(|| serde_json::to_string(&lyrics.lines))
            .transpose()?;
        let translation_json = lyrics
            .translation
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO lyrics (track_key, artist, title, album, duration_ms, source,
                                 provider_id, instrumental, lines_json, plain,
                                 translation_json, fetched_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(track_key) DO UPDATE SET
                 source           = excluded.source,
                 provider_id      = excluded.provider_id,
                 instrumental     = excluded.instrumental,
                 lines_json       = excluded.lines_json,
                 plain            = excluded.plain,
                 -- Uma tradução já baixada não pode ser apagada por uma
                 -- rebusca da letra original.
                 translation_json = COALESCE(excluded.translation_json, lyrics.translation_json),
                 fetched_at       = excluded.fetched_at",
            params![
                track.key(),
                track.artist,
                track.title,
                track.album,
                track.duration_ms.map(|d| d as i64),
                lyrics.source,
                lyrics.provider_id,
                lyrics.instrumental as i64,
                lines_json,
                lyrics.plain,
                translation_json,
                agora(),
            ],
        )?;

        // Achou agora: um fracasso anterior não vale mais.
        conn.execute(
            "DELETE FROM misses WHERE track_key = ?1",
            params![track.key()],
        )?;
        Ok(())
    }

    /// Quanto o cache já poupou de rede — o número que responde "ele guarda
    /// mesmo?" sem depender de acreditar.
    pub fn stats(&self) -> Result<CacheStats, CacheError> {
        let conn = self.conn.lock().unwrap();
        Ok(CacheStats {
            tracks: conn.query_row("SELECT COUNT(*) FROM lyrics", [], |r| r.get(0))?,
            synced: conn.query_row(
                "SELECT COUNT(*) FROM lyrics WHERE lines_json IS NOT NULL",
                [],
                |r| r.get(0),
            )?,
            pinned: conn.query_row("SELECT COUNT(*) FROM lyrics WHERE pinned = 1", [], |r| {
                r.get(0)
            })?,
            misses: conn.query_row("SELECT COUNT(*) FROM misses", [], |r| r.get(0))?,
        })
    }

    /// Marca a faixa como sem letra conhecida.
    pub fn mark_miss(&self, track_key: &str) -> Result<(), CacheError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO misses (track_key, tried_at) VALUES (?1, ?2)
             ON CONFLICT(track_key) DO UPDATE SET tried_at = excluded.tried_at",
            params![track_key, agora()],
        )?;
        Ok(())
    }

    /// Vale a pena tentar a rede de novo para esta faixa?
    pub fn should_retry(&self, track_key: &str) -> Result<bool, CacheError> {
        let conn = self.conn.lock().unwrap();
        let tried: Option<i64> = conn
            .query_row(
                "SELECT tried_at FROM misses WHERE track_key = ?1",
                params![track_key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(match tried {
            None => true,
            Some(t) => agora() - t > MISS_TTL_SECS,
        })
    }

    /// Fixa a letra para uso offline.
    ///
    /// Devolve `false` quando não há letra guardada para essa faixa — fixar o
    /// que não existe não faz sentido, e a interface precisa saber para não
    /// mostrar como fixada uma faixa que não está.
    pub fn set_pinned(&self, track_key: &str, pinned: bool) -> Result<bool, CacheError> {
        let conn = self.conn.lock().unwrap();
        let afetadas = conn.execute(
            "UPDATE lyrics SET pinned = ?2 WHERE track_key = ?1",
            params![track_key, pinned as i64],
        )?;
        Ok(afetadas > 0)
    }

    pub fn is_pinned(&self, track_key: &str) -> Result<bool, CacheError> {
        let conn = self.conn.lock().unwrap();
        let fixada: Option<i64> = conn
            .query_row(
                "SELECT pinned FROM lyrics WHERE track_key = ?1",
                params![track_key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(fixada == Some(1))
    }

    /// As letras fixadas, para a lista do modo offline.
    pub fn pinned(&self) -> Result<Vec<PinnedLyrics>, CacheError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT track_key, artist, title, lines_json IS NOT NULL AND lines_json != '[]'
             FROM lyrics WHERE pinned = 1
             ORDER BY artist COLLATE NOCASE, title COLLATE NOCASE",
        )?;
        let linhas = stmt.query_map([], |row| {
            Ok(PinnedLyrics {
                track_key: row.get(0)?,
                artist: row.get(1)?,
                title: row.get(2)?,
                synced: row.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(linhas.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faixa() -> Track {
        Track {
            title: "Infinito Particular".into(),
            artist: "Marisa Monte".into(),
            album: "Infinito Particular".into(),
            duration_ms: Some(251_000),
            source: "firefox".into(),
            ..Default::default()
        }
    }

    fn letra() -> Lyrics {
        Lyrics {
            lines: vec![LyricLine {
                at_ms: 4_040,
                text: "eis o melhor e o pior de mim".into(),
            }],
            plain: None,
            instrumental: false,
            source: "lrclib".into(),
            provider_id: Some("707294".into()),
            translation: None,
        }
    }

    #[test]
    fn guarda_e_devolve() {
        let c = Cache::in_memory().unwrap();
        let t = faixa();
        c.put(&t, &letra()).unwrap();

        let lida = c.get(&t.key()).unwrap().expect("devia estar no cache");
        assert_eq!(lida.lines.len(), 1);
        assert_eq!(lida.lines[0].at_ms, 4_040);
        assert_eq!(lida.provider_id.as_deref(), Some("707294"));
    }

    #[test]
    fn faixa_desconhecida_devolve_nada() {
        let c = Cache::in_memory().unwrap();
        assert!(c.get("nao|existe|").unwrap().is_none());
    }

    #[test]
    fn regravar_nao_apaga_traducao_ja_baixada() {
        let c = Cache::in_memory().unwrap();
        let t = faixa();

        let mut com_traducao = letra();
        com_traducao.translation = Some(vec![LyricLine {
            at_ms: 4_040,
            text: "here's the best and worst of me".into(),
        }]);
        c.put(&t, &com_traducao).unwrap();

        // Rebusca da letra original, sem tradução no payload.
        c.put(&t, &letra()).unwrap();

        let lida = c.get(&t.key()).unwrap().unwrap();
        assert!(lida.translation.is_some(), "a tradução foi perdida");
    }

    #[test]
    fn fixar_e_desafixar_uma_faixa_guardada() {
        let c = Cache::in_memory().unwrap();
        let t = faixa();
        c.put(&t, &letra()).unwrap();

        assert!(!c.is_pinned(&t.key()).unwrap(), "não nasce fixada");
        assert!(c.set_pinned(&t.key(), true).unwrap(), "fixou");
        assert!(c.is_pinned(&t.key()).unwrap());

        assert!(c.set_pinned(&t.key(), false).unwrap(), "achou a faixa");
        assert!(!c.is_pinned(&t.key()).unwrap());
    }

    /// Fixar o que não está no cache não pode passar por sucesso: a interface
    /// mostraria a faixa como disponível offline sem nada guardado.
    #[test]
    fn fixar_faixa_sem_letra_guardada_nao_aplica() {
        let c = Cache::in_memory().unwrap();
        assert!(!c.set_pinned("nem|existe|isso", true).unwrap());
        assert!(!c.is_pinned("nem|existe|isso").unwrap());
    }

    #[test]
    fn a_lista_de_fixadas_traz_metadados_e_ignora_o_resto() {
        let c = Cache::in_memory().unwrap();

        let fixada = faixa();
        c.put(&fixada, &letra()).unwrap();
        c.set_pinned(&fixada.key(), true).unwrap();

        let solta = Track {
            title: "Outra".into(),
            artist: "Alguém".into(),
            ..Default::default()
        };
        c.put(&solta, &letra()).unwrap();

        let lista = c.pinned().unwrap();
        assert_eq!(lista.len(), 1, "só a fixada entra na lista");
        assert_eq!(lista[0].artist, "Marisa Monte");
        assert_eq!(lista[0].title, "Infinito Particular");
        assert!(lista[0].synced, "esta tem letra sincronizada");
        assert_eq!(lista[0].track_key, fixada.key());
    }

    /// Letra sem sincronia também pode ser mantida offline — mas a lista tem
    /// que dizer qual é qual.
    #[test]
    fn a_lista_distingue_letra_sincronizada_de_texto_puro() {
        let c = Cache::in_memory().unwrap();
        let t = faixa();

        let mut so_texto = letra();
        so_texto.lines = Vec::new();
        so_texto.plain = Some("uma letra sem marcação de tempo".into());
        c.put(&t, &so_texto).unwrap();
        c.set_pinned(&t.key(), true).unwrap();

        let lista = c.pinned().unwrap();
        assert_eq!(lista.len(), 1);
        assert!(!lista[0].synced);
    }

    #[test]
    fn fracasso_e_lembrado_e_some_quando_acha() {
        let c = Cache::in_memory().unwrap();
        let t = faixa();

        assert!(c.should_retry(&t.key()).unwrap(), "sem registro, tenta");

        c.mark_miss(&t.key()).unwrap();
        assert!(
            !c.should_retry(&t.key()).unwrap(),
            "recém-falhou, não insiste"
        );

        c.put(&t, &letra()).unwrap();
        assert!(
            c.should_retry(&t.key()).unwrap(),
            "achou: o fracasso caducou"
        );
    }

    #[test]
    fn estatisticas_contam_o_que_importa() {
        let c = Cache::in_memory().unwrap();
        assert_eq!(c.stats().unwrap(), CacheStats::default());

        let t = faixa();
        c.put(&t, &letra()).unwrap();
        c.mark_miss("outra|faixa|sem letra").unwrap();

        let s = c.stats().unwrap();
        assert_eq!(s.tracks, 1);
        assert_eq!(s.synced, 1, "esta tem linhas sincronizadas");
        assert_eq!(s.pinned, 0);
        assert_eq!(s.misses, 1);

        c.set_pinned(&t.key(), true).unwrap();
        assert_eq!(c.stats().unwrap().pinned, 1);
    }

    #[test]
    fn instrumental_e_resposta_valida_no_cache() {
        let c = Cache::in_memory().unwrap();
        let t = faixa();
        let mut l = letra();
        l.lines.clear();
        l.instrumental = true;
        c.put(&t, &l).unwrap();

        let lida = c.get(&t.key()).unwrap().unwrap();
        assert!(lida.instrumental);
        assert!(!lida.is_empty());
    }
}
