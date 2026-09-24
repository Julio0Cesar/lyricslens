//! Reading the LRC format.
//!
//! LRC is a text file where each line carries one or more timestamps:
//!
//! ```text
//! [ti:Song]
//! [offset:+500]
//! [00:12.34]first line
//! [00:20.00][01:30.00]a chorus sung twice
//! ```
//!
//! Files in the wild are not clean. Parsing never fails: a line that makes no
//! sense is dropped, and what is left is still singable.

use std::time::Duration;

use crate::lyrics::{Line, LineKind, Lyrics};

/// Reads a whole LRC file.
pub fn parse(input: &str) -> Lyrics {
    let mut lyrics = Lyrics::default();

    for raw in input.lines() {
        let (tags, text) = split_tags(raw);
        if tags.is_empty() {
            continue;
        }

        let mut stamps = Vec::new();
        for tag in tags {
            match read_tag(&tag) {
                Tag::Time(at) => stamps.push(at),
                Tag::Meta(key, value) => apply_meta(&mut lyrics, &key, &value),
                Tag::Unknown => {}
            }
        }

        let text = text.trim();
        let kind = if text.is_empty() {
            LineKind::Instrumental
        } else {
            LineKind::Sung(text.to_owned())
        };
        for at in stamps {
            lyrics.lines.push(Line {
                at,
                kind: kind.clone(),
            });
        }
    }

    // Timestamps on one line need not be in order, and neither do the lines.
    lyrics.lines.sort_by_key(|line| line.at);
    lyrics
}

enum Tag {
    Time(Duration),
    Meta(String, String),
    Unknown,
}

/// Peels the leading `[...]` groups off a line and returns them with the rest.
fn split_tags(line: &str) -> (Vec<String>, &str) {
    let mut tags = Vec::new();
    let mut rest = line.trim_start();

    while let Some(stripped) = rest.strip_prefix('[') {
        let Some(end) = stripped.find(']') else {
            // An unclosed bracket: whatever follows is not a tag.
            break;
        };
        tags.push(stripped[..end].to_owned());
        rest = &stripped[end + 1..];
    }

    (tags, rest)
}

fn read_tag(tag: &str) -> Tag {
    let Some((left, right)) = tag.split_once(':') else {
        return Tag::Unknown;
    };

    // A timestamp starts with digits; a metadata key does not.
    if left.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return match read_time(left, right) {
            Some(at) => Tag::Time(at),
            None => Tag::Unknown,
        };
    }

    Tag::Meta(left.trim().to_ascii_lowercase(), right.trim().to_owned())
}

/// `mm` and `ss.xx`, where the fraction may be absent, two digits or three.
fn read_time(minutes: &str, seconds: &str) -> Option<Duration> {
    let minutes: u64 = minutes.trim().parse().ok()?;

    let (whole, fraction) = match seconds.split_once(['.', ':']) {
        Some((whole, fraction)) => (whole, fraction),
        None => (seconds, ""),
    };
    let whole: u64 = whole.trim().parse().ok()?;
    if whole >= 60 {
        return None;
    }

    let millis = match fraction.trim() {
        "" => 0,
        digits if digits.chars().all(|c| c.is_ascii_digit()) => {
            let value: u64 = digits.parse().ok()?;
            match digits.len() {
                1 => value * 100,
                2 => value * 10,
                3 => value,
                _ => return None,
            }
        }
        _ => return None,
    };

    Some(Duration::from_millis(
        (minutes * 60 + whole) * 1000 + millis,
    ))
}

fn apply_meta(lyrics: &mut Lyrics, key: &str, value: &str) {
    match key {
        "ti" => lyrics.title = non_empty(value),
        "ar" => lyrics.artist = non_empty(value),
        "al" => lyrics.album = non_empty(value),
        // The plus sign is allowed and means the same as no sign at all.
        "offset" => {
            if let Ok(offset) = value.trim_start_matches('+').parse() {
                lyrics.offset_ms = offset;
            }
        }
        _ => {}
    }
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: &Line) -> u64 {
        line.at.as_millis() as u64
    }

    fn text(line: &Line) -> &str {
        line.sung().expect("the line is sung")
    }

    #[test]
    fn reads_a_single_line() {
        let lyrics = parse("[00:12.34]hello");
        assert_eq!(lyrics.lines.len(), 1);
        assert_eq!(at(&lyrics.lines[0]), 12_340);
        assert_eq!(text(&lyrics.lines[0]), "hello");
    }

    #[test]
    fn one_line_with_several_timestamps_becomes_several_lines() {
        let lyrics = parse("[00:12.00][01:30.00]chorus");
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(at(&lyrics.lines[0]), 12_000);
        assert_eq!(at(&lyrics.lines[1]), 90_000);
        assert!(
            lyrics
                .lines
                .iter()
                .all(|line| line.sung() == Some("chorus"))
        );
    }

    #[test]
    fn lines_come_out_in_time_order_whatever_the_file_says() {
        let lyrics = parse("[00:30.00]second\n[00:10.00]first");
        assert_eq!(text(&lyrics.lines[0]), "first");
        assert_eq!(text(&lyrics.lines[1]), "second");
    }

    #[test]
    fn a_fraction_may_be_absent_two_digits_or_three() {
        assert_eq!(at(&parse("[00:05]x").lines[0]), 5_000);
        assert_eq!(at(&parse("[00:05.7]x").lines[0]), 5_700);
        assert_eq!(at(&parse("[00:05.25]x").lines[0]), 5_250);
        assert_eq!(at(&parse("[00:05.250]x").lines[0]), 5_250);
    }

    #[test]
    fn a_colon_may_separate_the_fraction_too() {
        assert_eq!(at(&parse("[00:05:25]x").lines[0]), 5_250);
    }

    #[test]
    fn minutes_run_past_an_hour() {
        assert_eq!(at(&parse("[75:00.00]x").lines[0]), 4_500_000);
    }

    #[test]
    fn metadata_tags_fill_the_header_and_leave_no_line() {
        let lyrics = parse("[ti:Paranoid Android]\n[ar:Radiohead]\n[al:OK Computer]");
        assert_eq!(lyrics.title.as_deref(), Some("Paranoid Android"));
        assert_eq!(lyrics.artist.as_deref(), Some("Radiohead"));
        assert_eq!(lyrics.album.as_deref(), Some("OK Computer"));
        assert!(lyrics.is_empty());
    }

    #[test]
    fn an_offset_is_read_with_or_without_its_plus_sign() {
        assert_eq!(parse("[offset:+500]").offset_ms, 500);
        assert_eq!(parse("[offset:500]").offset_ms, 500);
        assert_eq!(parse("[offset:-250]").offset_ms, -250);
    }

    #[test]
    fn a_meaningless_offset_leaves_the_default() {
        assert_eq!(parse("[offset:soon]").offset_ms, 0);
    }

    #[test]
    fn an_unknown_tag_is_ignored_without_dropping_the_line() {
        let lyrics = parse("[xx:whatever][00:01.00]still here");
        assert_eq!(lyrics.lines.len(), 1);
        assert_eq!(text(&lyrics.lines[0]), "still here");
    }

    #[test]
    fn a_line_with_no_timestamp_is_dropped() {
        assert!(parse("just some text").is_empty());
    }

    #[test]
    fn an_unclosed_bracket_does_not_eat_the_file() {
        let lyrics = parse("[00:01.00 broken\n[00:02.00]fine");
        assert_eq!(lyrics.lines.len(), 1);
        assert_eq!(text(&lyrics.lines[0]), "fine");
    }

    #[test]
    fn a_time_that_is_not_a_number_is_dropped() {
        assert!(parse("[ab:cd.ef]x").is_empty());
        assert!(parse("[00:xx.00]x").is_empty());
    }

    #[test]
    fn sixty_seconds_is_not_a_time() {
        assert!(parse("[00:60.00]x").is_empty());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_from_the_text() {
        assert_eq!(text(&parse("[00:01.00]   spaced   ").lines[0]), "spaced");
    }

    #[test]
    fn a_timestamp_with_no_words_is_an_instrumental_gap() {
        let lyrics = parse("[00:10.00]last words\n[00:14.00]\n[00:30.00]back again");
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.lines[1].kind, LineKind::Instrumental);
        assert_eq!(lyrics.lines[1].sung(), None);
    }

    #[test]
    fn a_gap_repeats_with_every_timestamp_on_its_line() {
        let lyrics = parse("[00:10.00][02:00.00]   ");
        assert_eq!(lyrics.lines.len(), 2);
        assert!(
            lyrics
                .lines
                .iter()
                .all(|line| line.kind == LineKind::Instrumental)
        );
    }

    #[test]
    fn an_empty_file_parses_to_nothing() {
        assert!(parse("").is_empty());
        assert!(parse("\n\n\n").is_empty());
    }
}
