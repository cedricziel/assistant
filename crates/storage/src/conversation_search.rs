//! Pure helpers for conversation search: query tokenizing, FTS5 query
//! building, accent- and case-insensitive folding, and highlight segments.
//!
//! The SQLite and in-memory conversation stores share these so that title
//! matching and the cross-message token rule cannot diverge.

use std::collections::{BTreeSet, HashMap, HashSet};

use anyhow::{Result, bail};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Maximum number of whitespace-separated tokens in one search query.
pub const MAX_QUERY_TOKENS: usize = 8;

/// Marks the start of a hit in an FTS5 `snippet()`.
pub const HIGHLIGHT_START: char = '\u{E000}';
/// Marks the end of a hit in an FTS5 `snippet()`.
pub const HIGHLIGHT_END: char = '\u{E001}';

/// A run of text, flagged when it matched a query token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSegment {
    pub text: String,
    pub highlighted: bool,
}

/// A title that satisfies at least one query token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleMatch {
    pub segments: Vec<SearchSegment>,
    pub satisfied: BTreeSet<String>,
}

/// Splits a search query on whitespace into at most [`MAX_QUERY_TOKENS`] tokens.
///
/// Blank input yields no tokens; the caller decides how to report that.
pub fn tokenize_query(input: &str) -> Result<Vec<String>> {
    let tokens: Vec<String> = input.split_whitespace().map(str::to_string).collect();
    if tokens.len() > MAX_QUERY_TOKENS {
        bail!("search query has more than {MAX_QUERY_TOKENS} words");
    }
    Ok(tokens)
}

/// Whether a token is too short to scan message text with.
pub fn is_short_token(token: &str) -> bool {
    token.chars().count() < 2
}

/// Builds the FTS5 `MATCH` expression for the message-searchable tokens.
///
/// Each token becomes a quoted prefix term (`"tok"*`) with `"` doubled, so
/// FTS5 operators in user input stay literal. Terms are OR-ed. One-character
/// tokens are dropped; `None` means there is nothing to search messages for.
pub fn fts_query(tokens: &[String]) -> Option<String> {
    let terms: Vec<String> = tokens
        .iter()
        .filter(|t| !is_short_token(t))
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" OR "))
    }
}

/// Lowercases text and strips diacritics (NFD, then drop combining marks).
///
/// Returns the folded string and, for every folded char, the index of the
/// source char it came from, so matches can be mapped back to the original.
pub fn fold(text: &str) -> (String, Vec<usize>) {
    let mut folded = String::with_capacity(text.len());
    let mut source_index = Vec::with_capacity(text.len());
    for (index, c) in text.chars().enumerate() {
        for lower in c.to_lowercase() {
            for d in lower.nfd().filter(|d| !is_combining_mark(*d)) {
                folded.push(d);
                source_index.push(index);
            }
        }
    }
    (folded, source_index)
}

/// Matches `tokens` as folded substrings of `title` and highlights the
/// matched stretches of the original text.
///
/// Returns the segments and the tokens the title contains, or `None` when it
/// contains none of them.
pub fn title_segments(title: &str, tokens: &[String]) -> Option<TitleMatch> {
    let (folded, source_index) = fold(title);
    let folded: Vec<char> = folded.chars().collect();
    let source_len = title.chars().count();

    let mut satisfied = BTreeSet::new();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for token in tokens {
        let needle: Vec<char> = fold(token).0.chars().collect();
        if needle.is_empty() || needle.len() > folded.len() {
            continue;
        }
        let mut found = false;
        for start in 0..=folded.len() - needle.len() {
            if folded[start..start + needle.len()] != needle[..] {
                continue;
            }
            found = true;
            let end = start + needle.len();
            // Extend over trailing source chars that folded away entirely
            // (combining marks), so a highlight never strands a mark.
            let next_kept = source_index.get(end).copied().unwrap_or(source_len);
            ranges.push((
                source_index[start],
                next_kept.max(source_index[end - 1] + 1),
            ));
        }
        if found {
            satisfied.insert(token.clone());
        }
    }
    if satisfied.is_empty() {
        return None;
    }

    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }

    let chars: Vec<char> = title.chars().collect();
    let mut segments = Vec::new();
    let mut cursor = 0;
    for (start, end) in merged {
        if start > cursor {
            segments.push(segment(&chars[cursor..start], false));
        }
        segments.push(segment(&chars[start..end], true));
        cursor = end;
    }
    if cursor < chars.len() {
        segments.push(segment(&chars[cursor..], false));
    }
    Some(TitleMatch {
        segments,
        satisfied,
    })
}

fn segment(chars: &[char], highlighted: bool) -> SearchSegment {
    SearchSegment {
        text: chars.iter().collect(),
        highlighted,
    }
}

/// Splits an FTS5 `snippet()` that marks hits with U+E000 / U+E001 into
/// segments. Markers never appear in the output text.
pub fn segments_from_snippet(snippet: &str) -> Vec<SearchSegment> {
    let mut segments = Vec::new();
    let mut buffer = String::new();
    let mut highlighted = false;
    let mut flush = |buffer: &mut String, highlighted: bool| {
        if !buffer.is_empty() {
            segments.push(SearchSegment {
                text: std::mem::take(buffer),
                highlighted,
            });
        }
    };
    for c in snippet.chars() {
        match c {
            HIGHLIGHT_START => {
                flush(&mut buffer, highlighted);
                highlighted = true;
            }
            HIGHLIGHT_END => {
                flush(&mut buffer, highlighted);
                highlighted = false;
            }
            _ => buffer.push(c),
        }
    }
    flush(&mut buffer, highlighted);
    segments
}

/// Returns, sorted, the conversations in which every required token is
/// satisfied by the title or by at least one message (not necessarily the
/// same one).
///
/// One-character tokens are not required when any token has two or more
/// characters. When none does, every token is required and only titles count.
pub fn satisfied_conversations(
    tokens: &[String],
    title_tokens_by_conversation: &HashMap<String, HashSet<String>>,
    message_tokens_by_conversation: &HashMap<String, HashSet<String>>,
) -> Vec<String> {
    let has_long = tokens.iter().any(|t| !is_short_token(t));
    let required: Vec<&String> = tokens
        .iter()
        .filter(|t| !has_long || !is_short_token(t))
        .collect();
    if required.is_empty() {
        return Vec::new();
    }

    let ids: BTreeSet<&String> = title_tokens_by_conversation
        .keys()
        .chain(message_tokens_by_conversation.keys())
        .collect();
    ids.into_iter()
        .filter(|id| {
            let in_title = title_tokens_by_conversation.get(*id);
            let in_messages = message_tokens_by_conversation.get(*id);
            required.iter().all(|t| {
                in_title.is_some_and(|set| set.contains(*t))
                    || (has_long && in_messages.is_some_and(|set| set.contains(*t)))
            })
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(input: &str) -> Vec<String> {
        input.split_whitespace().map(str::to_string).collect()
    }

    fn seg(text: &str, highlighted: bool) -> SearchSegment {
        SearchSegment {
            text: text.to_string(),
            highlighted,
        }
    }

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn map(entries: &[(&str, &[&str])]) -> HashMap<String, HashSet<String>> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), set(v)))
            .collect()
    }

    // -- tokenize_query / fts_query --

    #[test]
    fn fts_query_prefix_quotes_each_token() {
        assert_eq!(
            fts_query(&toks("lisbon hotel")).as_deref(),
            Some(r#""lisbon"* OR "hotel"*"#)
        );
    }

    #[test]
    fn fts_query_doubles_quotes_and_keeps_operators_literal() {
        let q = fts_query(&toks(r#""foo" OR bar* (x NEAR"#)).unwrap();
        assert_eq!(q, r#""""foo"""* OR "OR"* OR "bar*"* OR "(x"* OR "NEAR"*"#);
    }

    #[test]
    fn fts_query_drops_one_character_tokens() {
        assert_eq!(
            fts_query(&toks("a lisbon q")).as_deref(),
            Some(r#""lisbon"*"#)
        );
    }

    #[test]
    fn fts_query_is_none_for_blank_or_all_short_input() {
        assert_eq!(fts_query(&[]), None);
        assert_eq!(fts_query(&toks("   ")), None);
        assert_eq!(fts_query(&toks("a b")), None);
    }

    #[test]
    fn tokenize_query_splits_on_whitespace() {
        assert_eq!(
            tokenize_query("  lisbon \t hotel\n").unwrap(),
            toks("lisbon hotel")
        );
        assert!(tokenize_query("   ").unwrap().is_empty());
    }

    #[test]
    fn tokenize_query_rejects_more_than_eight_tokens() {
        assert_eq!(tokenize_query("a b c d e f g h").unwrap().len(), 8);
        assert!(tokenize_query("a b c d e f g h i").is_err());
    }

    // -- fold --

    #[test]
    fn fold_lowercases_and_strips_diacritics() {
        assert_eq!(fold("Ärztetermin").0, "arztetermin");
        assert_eq!(fold("Lisbôa").0, "lisboa");
    }

    #[test]
    fn fold_maps_each_folded_char_to_its_source_char() {
        assert_eq!(fold("Lisbôa").1, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(fold("Ä").1, vec![0]);
        // `İ` lowercases to `i` plus a combining dot, which is dropped.
        let (folded, map) = fold("aİb");
        assert_eq!(folded, "aib");
        assert_eq!(map, vec![0, 1, 2]);
    }

    #[test]
    fn fold_drops_combining_marks_of_decomposed_input() {
        let (folded, map) = fold("o\u{302}x");
        assert_eq!(folded, "ox");
        assert_eq!(map, vec![0, 2]);
    }

    // -- title_segments --

    #[test]
    fn title_segments_highlight_original_text_for_folded_match() {
        let m = title_segments("Ärztetermin", &toks("ärzte")).unwrap();
        assert_eq!(m.segments, vec![seg("Ärzte", true), seg("termin", false)]);
        assert_eq!(m.satisfied, ["ärzte".to_string()].into());

        let m = title_segments("Lisbôa", &toks("lisboa")).unwrap();
        assert_eq!(m.segments, vec![seg("Lisbôa", true)]);
    }

    #[test]
    fn title_segments_highlight_only_tokens_the_title_contains() {
        let m = title_segments("Lisbon trip", &toks("lisbon hotel")).unwrap();
        assert_eq!(m.segments, vec![seg("Lisbon", true), seg(" trip", false)]);
        assert_eq!(m.satisfied, ["lisbon".to_string()].into());
    }

    #[test]
    fn title_segments_match_substrings_everywhere_and_merge_overlaps() {
        let m = title_segments("Trip to Lisbon", &toks("t li lis")).unwrap();
        assert_eq!(
            m.segments,
            vec![
                seg("T", true),
                seg("rip ", false),
                seg("t", true),
                seg("o ", false),
                seg("Lis", true),
                seg("bon", false),
            ]
        );
    }

    #[test]
    fn title_segments_none_when_no_token_matches() {
        assert_eq!(title_segments("Lisbon trip", &toks("hotel")), None);
        assert_eq!(title_segments("", &toks("hotel")), None);
        assert_eq!(title_segments("Lisbon", &[]), None);
    }

    #[test]
    fn title_segments_concatenate_to_the_title() {
        let title = "aİb Ärzte ô";
        let m = title_segments(title, &toks("ib arzte o")).unwrap();
        let joined: String = m.segments.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, title);
    }

    #[test]
    fn title_segments_keep_decomposed_marks_inside_the_highlight() {
        let m = title_segments("Lisbo\u{302}a", &toks("lisbo")).unwrap();
        assert_eq!(m.segments, vec![seg("Lisbo\u{302}", true), seg("a", false)]);
    }

    // -- segments_from_snippet --

    #[test]
    fn segments_from_snippet_split_on_markers() {
        assert_eq!(
            segments_from_snippet("The \u{E000}Lisbon\u{E001} itinerary is ready"),
            vec![
                seg("The ", false),
                seg("Lisbon", true),
                seg(" itinerary is ready", false)
            ]
        );
    }

    #[test]
    fn segments_from_snippet_without_markers_is_one_plain_segment() {
        assert_eq!(
            segments_from_snippet("…no hits…"),
            vec![seg("…no hits…", false)]
        );
        assert!(segments_from_snippet("").is_empty());
    }

    #[test]
    fn segments_from_snippet_handles_edges_and_adjacent_hits() {
        assert_eq!(
            segments_from_snippet("\u{E000}a\u{E001}\u{E000}b\u{E001}"),
            vec![seg("a", true), seg("b", true)]
        );
    }

    #[test]
    fn segments_from_snippet_never_emit_markers() {
        let got = segments_from_snippet("x\u{E001}y\u{E000}z");
        assert!(!got.is_empty());
        for s in got {
            assert!(!s.text.contains('\u{E000}') && !s.text.contains('\u{E001}'));
        }
    }

    // -- satisfied_conversations --

    #[test]
    fn tokens_may_come_from_different_messages() {
        // `a` has one message with "lisbon" and another with "hotel"; the
        // caller unions the per-message token sets per conversation.
        let messages = map(&[("a", &["lisbon", "hotel"]), ("b", &["lisbon"])]);
        let got = satisfied_conversations(&toks("lisbon hotel"), &HashMap::new(), &messages);
        assert_eq!(got, vec!["a".to_string()]);
    }

    #[test]
    fn title_and_message_can_split_the_tokens() {
        let titles = map(&[("t", &["lisbon"])]);
        let messages = map(&[("t", &["hotel"])]);
        let got = satisfied_conversations(&toks("lisbon hotel"), &titles, &messages);
        assert_eq!(got, vec!["t".to_string()]);
    }

    #[test]
    fn one_character_tokens_are_not_required_next_to_a_longer_token() {
        let messages = map(&[("a", &["lisbon"])]);
        let got = satisfied_conversations(&toks("lisbon q"), &HashMap::new(), &messages);
        assert_eq!(got, vec!["a".to_string()]);
    }

    #[test]
    fn one_character_tokens_are_required_against_titles_only() {
        let titles = map(&[("t", &["q"])]);
        let messages = map(&[("m", &["q"])]);
        let got = satisfied_conversations(&toks("q"), &titles, &messages);
        assert_eq!(got, vec!["t".to_string()]);

        let got = satisfied_conversations(&toks("q x"), &map(&[("t", &["q"])]), &messages);
        assert!(got.is_empty(), "every short token is required: {got:?}");
    }

    #[test]
    fn empty_required_set_returns_nothing() {
        let messages = map(&[("a", &["lisbon"])]);
        assert!(satisfied_conversations(&[], &HashMap::new(), &messages).is_empty());
    }

    #[test]
    fn result_is_sorted_and_deduplicated() {
        let titles = map(&[("b", &["lisbon"]), ("a", &["lisbon"])]);
        let messages = map(&[("a", &["lisbon"])]);
        let got = satisfied_conversations(&toks("lisbon"), &titles, &messages);
        assert_eq!(got, vec!["a".to_string(), "b".to_string()]);
    }
}
