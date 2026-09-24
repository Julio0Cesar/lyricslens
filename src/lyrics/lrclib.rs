//! The LRCLIB client.
//!
//! LRCLIB is free, has no API key, and asks callers to identify themselves.
//! Two endpoints matter: `/api/get`, which wants the exact signature of a
//! recording, and `/api/search`, for when the signature is slightly off —
//! which it usually is, coming from a video title.

use std::time::Duration;

use serde::Deserialize;

use super::lrc;
use super::normalize::Query;
use crate::error::Error;
use crate::lyrics::Lyrics;

const BASE: &str = "https://lrclib.net";

/// Identifying the caller is the one thing the service asks for in return.
const AGENT: &str = concat!(
    "LyricsLens/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/Julio0Cesar/lyricslens)"
);

/// How far a search result's duration may sit from the one being played.
const TOLERANCE: Duration = Duration::from_secs(4);

/// Lyrics as they came, with the text they were parsed from. The raw LRC is
/// what goes to the cache: it survives a change to the parser.
#[derive(Debug, Clone)]
pub struct Found {
    pub lyrics: Lyrics,
    pub lrc: String,
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base: String,
}

/// One recording as LRCLIB describes it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    synced_lyrics: Option<String>,
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            http: reqwest::Client::builder().user_agent(AGENT).build()?,
            base: BASE.to_owned(),
        })
    }

    /// Synced lyrics for what is playing, or `None` when the service has none.
    ///
    /// Plain lyrics are deliberately ignored: without timestamps there is
    /// nothing for the overlay to follow.
    pub async fn lyrics(
        &self,
        query: &Query,
        duration: Option<Duration>,
    ) -> Result<Option<Found>, Error> {
        if let Some(record) = self.exact(query, duration).await?
            && let Some(found) = synced(&record)
        {
            return Ok(Some(found));
        }

        if let Some(record) = self.searched(query, duration).await? {
            return Ok(synced(&record));
        }

        // A last try without the artist. Players hand over whole credit lists
        // — "A, B, C, feat. D" — and no catalogue is indexed under those.
        if query.artist.is_some() {
            let title_only = Query {
                artist: None,
                title: query.title.clone(),
            };
            if let Some(record) = self.searched(&title_only, duration).await? {
                return Ok(synced(&record));
            }
        }

        Ok(None)
    }

    async fn exact(
        &self,
        query: &Query,
        duration: Option<Duration>,
    ) -> Result<Option<Record>, Error> {
        let Some(artist) = query.artist.as_deref() else {
            // /api/get needs both names; without an artist it can only miss.
            return Ok(None);
        };

        let mut params = vec![
            ("artist_name", artist.to_owned()),
            ("track_name", query.title.clone()),
        ];
        if let Some(duration) = duration {
            params.push(("duration", duration.as_secs().to_string()));
        }

        let response = self
            .http
            .get(format!("{}/api/get", self.base))
            .query(&params)
            .send()
            .await?;

        // A miss is a 404, and the service answers 503 for signatures it cannot
        // place at all. Neither is a reason to give up: the search endpoint is
        // more forgiving, and that is the next thing tried.
        if !response.status().is_success() {
            tracing::debug!(status = %response.status(), "no exact match");
            return Ok(None);
        }
        Ok(Some(response.json().await?))
    }

    async fn searched(
        &self,
        query: &Query,
        duration: Option<Duration>,
    ) -> Result<Option<Record>, Error> {
        let mut params = vec![("track_name", query.title.clone())];
        if let Some(artist) = query.artist.as_deref() {
            params.push(("artist_name", artist.to_owned()));
        }

        let response = self
            .http
            .get(format!("{}/api/search", self.base))
            .query(&params)
            .send()
            .await?;

        // The service answers 5xx for queries it cannot make sense of. That is
        // an answer about the song, not a failure worth passing up: the caller
        // would turn it into an error message where "not found" belongs.
        if !response.status().is_success() {
            tracing::debug!(status = %response.status(), "the search found nothing");
            return Ok(None);
        }

        Ok(best(response.json().await?, duration))
    }
}

/// Picks the result closest in length to what is playing.
///
/// Search returns covers, live versions and remixes under the same name; the
/// duration is what tells one recording from another.
fn best(records: Vec<Record>, duration: Option<Duration>) -> Option<Record> {
    let mut with_lyrics = records
        .into_iter()
        .filter(|record| synced(record).is_some());

    let picked = match duration {
        None => with_lyrics.next(),
        Some(duration) => {
            let target = duration.as_secs_f64();
            with_lyrics
                .filter(|record| match record.duration {
                    Some(seconds) => (seconds - target).abs() <= TOLERANCE.as_secs_f64(),
                    None => false,
                })
                .min_by(|a, b| {
                    let distance =
                        |record: &Record| (record.duration.unwrap_or_default() - target).abs();
                    distance(a).total_cmp(&distance(b))
                })
        }
    };

    // Worth logging: when the overlay shows the wrong version of a song, this
    // line says which recording the search settled on.
    if let Some(record) = &picked {
        tracing::debug!(
            track = %record.track_name,
            artist = %record.artist_name,
            seconds = ?record.duration,
            "picked from search"
        );
    }
    picked
}

/// The synced lyrics of a record, parsed, when it has any worth showing.
fn synced(record: &Record) -> Option<Found> {
    if record.instrumental {
        return None;
    }
    let lrc = record.synced_lyrics.as_deref()?;
    let lyrics = lrc::parse(lrc);
    (!lyrics.is_empty()).then(|| Found {
        lyrics,
        lrc: lrc.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, seconds: f64, lyrics: Option<&str>) -> Record {
        Record {
            track_name: name.to_owned(),
            artist_name: "Someone".to_owned(),
            duration: Some(seconds),
            instrumental: false,
            synced_lyrics: lyrics.map(str::to_owned),
        }
    }

    const SYNCED: &str = "[00:01.00]a line";

    #[test]
    fn the_agent_names_the_app_and_its_version() {
        assert!(AGENT.starts_with("LyricsLens/"));
        assert!(AGENT.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn a_record_without_synced_lyrics_is_no_use() {
        assert!(synced(&record("x", 100.0, None)).is_none());
    }

    #[test]
    fn an_instrumental_record_is_no_use_either() {
        let mut instrumental = record("x", 100.0, Some(SYNCED));
        instrumental.instrumental = true;
        assert!(synced(&instrumental).is_none());
    }

    #[test]
    fn the_closest_duration_wins() {
        let records = vec![
            record("live", 240.0, Some(SYNCED)),
            record("album", 200.0, Some(SYNCED)),
        ];
        let picked = best(records, Some(Duration::from_secs(201))).expect("a match");
        assert_eq!(picked.track_name, "album");
    }

    #[test]
    fn a_recording_of_a_very_different_length_is_not_the_same_song() {
        let records = vec![record("extended mix", 600.0, Some(SYNCED))];
        assert!(best(records, Some(Duration::from_secs(200))).is_none());
    }

    #[test]
    fn without_a_duration_the_first_usable_result_is_taken() {
        let records = vec![
            record("no lyrics", 200.0, None),
            record("usable", 999.0, Some(SYNCED)),
        ];
        let picked = best(records, None).expect("a match");
        assert_eq!(picked.track_name, "usable");
    }

    #[test]
    fn nothing_usable_means_nothing() {
        assert!(best(Vec::new(), None).is_none());
        assert!(
            best(
                vec![record("x", 200.0, None)],
                Some(Duration::from_secs(200))
            )
            .is_none()
        );
    }
}
