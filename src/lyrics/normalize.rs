//! Turning what a player reports into something worth searching for.
//!
//! A music player hands over a clean artist and title. A browser playing
//! YouTube hands over the video's name and the channel's name, which is a
//! different thing: `Artist - Title (Official Video) [HD] ft. Someone`, from a
//! channel called `ArtistVEVO`. Searching for that finds nothing.

use std::sync::LazyLock;

use regex::Regex;

use crate::media::Track;

/// What to ask a lyrics service for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub artist: Option<String>,
    pub title: String,
}

/// Words that only ever describe the upload, never the song. Uploaders stack
/// them — `(Official Video Remastered)`, `[Official Music Video HD]` — so the
/// whole group has to be nothing but these.
static NOISE: LazyLock<Regex> = LazyLock::new(|| {
    const WORD: &str = r"(?:
        official | music | lyric[s]? | audio | video | visuali[sz]er
      | remaster(?:ed)? | explicit | full | album | live\s+performance
      | hd | hq | 4k | 8k | 1080p | 720p | m/?v | color\s+coded
      | \d{4}
    )";
    Regex::new(&format!(
        r"(?ix)^[\s\-/,|]*{WORD}(?:[\s\-/,|]+{WORD})*[\s\-/,|]*$"
    ))
    .expect("the pattern is valid")
});

/// `feat. X`, in any of the spellings uploaders use.
static FEATURING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[\(\[]?\s*\b(?:feat\.?|ft\.?|featuring|with)\b\s*([^)\]\|]*)[\)\]]?\s*$")
        .expect("the pattern is valid")
});

/// A channel name is not an artist name.
static CHANNEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(vevo$|\s-\s*topic$|official$|^youtube$|records$|\bTV$)")
        .expect("the pattern is valid")
});

/// Every dash a title might be split on. Uploaders use all of them.
const DASHES: [&str; 4] = [" - ", " – ", " — ", " ‒ "];

/// Reads a track the way a lyrics service would need it.
pub fn from_track(track: &Track) -> Query {
    clean(
        track.artists.first().map(String::as_str),
        track.title.as_deref().unwrap_or_default(),
    )
}

/// The whole cleanup, from the two strings a player gives us.
pub fn clean(artist: Option<&str>, title: &str) -> Query {
    let mut title = strip_brackets(title);
    title = strip_tail(&title);
    title = FEATURING.replace(&title, "").into_owned();

    let artist = artist.map(str::trim).filter(|name| !name.is_empty());
    let (artist, title) = match split_on_dash(&title) {
        // The title carries the artist. Trust it over a channel name, and over
        // nothing at all.
        Some((left, right)) if artist.is_none_or(is_channel) => (Some(left), right),
        // Both agree, which is the common case on `Artist - Title` uploads.
        Some((left, right)) if same_name(&left, artist.unwrap_or_default()) => (Some(left), right),
        _ => (artist.map(strip_channel_suffix), title),
    };

    Query {
        artist: artist
            .map(|name| primary(&trim_quotes(&name)))
            .filter(|name| !name.is_empty()),
        title: trim_quotes(&title),
    }
}

/// The first name of a credit list.
///
/// Players hand over every credit in one string — `A, B, C & D` — and no
/// lyrics catalogue is indexed under that. The first one is the one that finds
/// the song.
fn primary(artist: &str) -> String {
    let cut = artist
        .split_once(',')
        .or_else(|| artist.split_once(" & "))
        .or_else(|| artist.split_once(" feat"))
        .map_or(artist, |(first, _)| first);
    cut.trim().to_owned()
}

/// Drops `(...)` and `[...]` groups that say nothing about the song.
fn strip_brackets(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut depth = 0usize;
    let mut group = String::new();

    for c in title.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                if depth == 1 {
                    group.clear();
                    continue;
                }
            }
            ')' | ']' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    // A group that is not noise belongs to the song's name:
                    // `(Acoustic)` and `(Remix)` change which recording it is.
                    if !NOISE.is_match(&group) {
                        out.push('(');
                        out.push_str(group.trim());
                        out.push(')');
                    }
                    continue;
                }
            }
            _ => {}
        }
        if depth == 0 {
            out.push(c);
        } else {
            group.push(c);
        }
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Removes a trailing `| HD`, `| Official Video`, and the like.
fn strip_tail(title: &str) -> String {
    let mut title = title.trim().to_owned();
    while let Some((head, tail)) = title.rsplit_once('|') {
        if !NOISE.is_match(tail) {
            break;
        }
        title = head.trim().to_owned();
    }
    title
}

fn split_on_dash(title: &str) -> Option<(String, String)> {
    let (left, right) = DASHES
        .iter()
        .find_map(|dash| title.split_once(dash))
        // `Artist-Title` with no spaces is ambiguous: `Jay-Z` exists.
        ?;
    let (left, right) = (left.trim(), right.trim());
    (!left.is_empty() && !right.is_empty()).then(|| (left.to_owned(), right.to_owned()))
}

fn is_channel(name: &str) -> bool {
    CHANNEL.is_match(name.trim())
}

fn strip_channel_suffix(name: &str) -> String {
    CHANNEL.replace(name.trim(), "").trim().to_owned()
}

fn same_name(a: &str, b: &str) -> bool {
    let fold = |s: &str| {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    };
    !b.is_empty() && fold(a) == fold(b)
}

fn trim_quotes(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '-' || c == '–')
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real uploads, and what a lyrics service needs them to become.
    #[test]
    fn a_table_of_noisy_titles() {
        let cases = [
            (
                None,
                "Radiohead - Paranoid Android (Official Video)",
                "Radiohead",
                "Paranoid Android",
            ),
            (
                Some("Michael Jackson"),
                "Michael Jackson - Billie Jean (Official Video) [HD]",
                "Michael Jackson",
                "Billie Jean",
            ),
            (
                Some("QueenOfficial"),
                "Queen – Bohemian Rhapsody (Official Video Remastered)",
                "Queen",
                "Bohemian Rhapsody",
            ),
            (
                Some("Daft Punk - Topic"),
                "Daft Punk - Get Lucky (Official Audio) ft. Pharrell Williams",
                "Daft Punk",
                "Get Lucky",
            ),
            (
                Some("AdeleVEVO"),
                "Adele - Hello | Lyrics",
                "Adele",
                "Hello",
            ),
            (
                None,
                "Linkin Park - Numb [Official Music Video]",
                "Linkin Park",
                "Numb",
            ),
            (
                Some("Gorillaz"),
                "Feel Good Inc. (Official Video)",
                "Gorillaz",
                "Feel Good Inc.",
            ),
            (
                None,
                "Tame Impala - The Less I Know The Better | 4K",
                "Tame Impala",
                "The Less I Know The Better",
            ),
        ];

        for (artist, title, want_artist, want_title) in cases {
            let query = clean(artist, title);
            assert_eq!(
                (query.artist.as_deref(), query.title.as_str()),
                (Some(want_artist), want_title),
                "on {title:?}"
            );
        }
    }

    #[test]
    fn a_clean_player_is_left_alone() {
        let query = clean(Some("Radiohead"), "Paranoid Android");
        assert_eq!(query.artist.as_deref(), Some("Radiohead"));
        assert_eq!(query.title, "Paranoid Android");
    }

    #[test]
    fn a_parenthesis_that_names_the_recording_stays() {
        assert_eq!(
            clean(None, "Nirvana - Come As You Are (Unplugged)").title,
            "Come As You Are (Unplugged)"
        );
        assert_eq!(
            clean(None, "Avicii - Levels (Radio Edit)").title,
            "Levels (Radio Edit)"
        );
    }

    #[test]
    fn a_hyphenated_name_is_not_split() {
        let query = clean(Some("Jay-Z"), "Jay-Z");
        assert_eq!(query.title, "Jay-Z");
    }

    #[test]
    fn a_title_with_no_artist_anywhere_keeps_the_artist_empty() {
        let query = clean(None, "some random upload");
        assert_eq!(query.artist, None);
        assert_eq!(query.title, "some random upload");
    }

    #[test]
    fn only_the_first_of_a_credit_list_is_searched_for() {
        let query = clean(
            Some("Dj Brenno, SAM SAM, Manzzy, MC Carol"),
            "Ilha de Capri",
        );
        assert_eq!(query.artist.as_deref(), Some("Dj Brenno"));

        let query = clean(Some("Calvin Harris & Dua Lipa"), "One Kiss");
        assert_eq!(query.artist.as_deref(), Some("Calvin Harris"));
    }

    #[test]
    fn an_empty_title_survives() {
        assert_eq!(clean(None, ""), Query::default());
    }
}
