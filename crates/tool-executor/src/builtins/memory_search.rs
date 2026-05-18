//! Builtin handler for the `memory-search` tool.
//!
//! Performs a hybrid FTS5 + cosine-similarity vector search over indexed
//! memory chunks, with four quality layers:
//!
//! 1. **Query expansion** — extract keywords from conversational queries so
//!    FTS5 sees useful terms even when the user phrases things naturally.
//! 2. **Hybrid scoring** — 70 % cosine similarity + 30 % normalised BM25 rank
//!    (vector similarity dominates when embeddings are available).
//! 3. **Temporal decay** — daily notes lose relevance with a 30-day half-life;
//!    evergreen files (`MEMORY.md`, `SOUL.md`, …) are never penalised.
//! 4. **MMR re-ranking** — Maximal Marginal Relevance (λ=0.7) reduces
//!    redundancy in the returned set.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::{LlmProvider, ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use assistant_storage::{MemoryChunkStore, StorageLayer};
use async_trait::async_trait;
use chrono::NaiveDate;
use whatlang::Lang;

const DEFAULT_LIMIT: i64 = 5;

/// Weight given to vector (cosine) similarity in the hybrid score.
const VECTOR_WEIGHT: f64 = 0.7;
/// Weight given to normalised BM25 rank in the hybrid score.
const BM25_WEIGHT: f64 = 0.3;

/// Half-life for temporal decay of daily notes, in days.
const DECAY_HALF_LIFE_DAYS: f64 = 30.0;

/// MMR trade-off: higher → more relevant, lower → more diverse.
const MMR_LAMBDA: f64 = 0.7;

// ---------------------------------------------------------------------------
// Stop words for query expansion
// ---------------------------------------------------------------------------

/// Memory-search–specific vague terms filtered regardless of language.
const MEMORY_STOP_WORDS: &[&str] = &[
    "thing",
    "things",
    "discussed",
    "talked",
    "mentioned",
    "remember",
    "recall",
    "tell",
    "told",
    "yesterday",
    "today",
    "recently",
    "last",
    "ago",
    "back",
];

/// Return the stop word set for the detected language of `text`.
///
/// Uses `whatlang` to identify the language, then fetches the per-language
/// list from the `stop-words` crate.  Always also folds in English stop words
/// (memory files are often English even when the query isn't) and the
/// memory-specific vague terms above.
fn stop_words_for(text: &str) -> HashSet<String> {
    let lang_code = whatlang::detect_lang(text)
        .and_then(whatlang_to_iso639_1)
        .unwrap_or("en");

    let mut words: HashSet<String> = stop_words::get(lang_code)
        .iter()
        .map(|w| w.to_lowercase())
        .collect();

    // Blend in English so queries mixing languages still filter common terms.
    if lang_code != "en" {
        for w in stop_words::get("en") {
            words.insert(w.to_lowercase());
        }
    }

    for w in MEMORY_STOP_WORDS {
        words.insert(w.to_lowercase());
    }

    words
}

/// Map a `whatlang::Lang` to an ISO 639-1 code accepted by `stop-words`.
///
/// Only languages covered by the `stop-words` ISO feature set are listed;
/// everything else returns `None` (caller falls back to English).
fn whatlang_to_iso639_1(lang: Lang) -> Option<&'static str> {
    match lang {
        Lang::Ara => Some("ar"),
        Lang::Dan => Some("da"),
        Lang::Nld => Some("nl"),
        Lang::Eng => Some("en"),
        Lang::Fin => Some("fi"),
        Lang::Fra => Some("fr"),
        Lang::Deu => Some("de"),
        Lang::Ell => Some("el"),
        Lang::Hun => Some("hu"),
        Lang::Ind => Some("id"),
        Lang::Ita => Some("it"),
        Lang::Nob => Some("no"),
        Lang::Por => Some("pt"),
        Lang::Ron => Some("ro"),
        Lang::Rus => Some("ru"),
        Lang::Spa => Some("es"),
        Lang::Swe => Some("sv"),
        Lang::Tur => Some("tr"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// A scored search candidate ready for MMR selection.
struct Candidate {
    chunk_id: i64,
    file_path: String,
    content: String,
    score: f64,
    /// Dense embedding for MMR diversity computation (may be absent).
    embedding: Option<Vec<f32>>,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub struct MemorySearchHandler {
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
}

impl MemorySearchHandler {
    pub fn new(storage: Arc<StorageLayer>, llm: Arc<dyn LlmProvider>) -> Self {
        Self { storage, llm }
    }
}

#[async_trait]
impl ToolHandler for MemorySearchHandler {
    fn name(&self) -> &str {
        "memory-search"
    }

    fn description(&self) -> &str {
        "Search indexed memory chunks using full-text search and optionally vector similarity. Returns the most relevant memory excerpts."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 20,
                    "description": "Maximum number of results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let query = match params.get("query").and_then(|v| v.as_str()) {
            Some(q) => q.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'query'")),
        };
        let limit = params
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(DEFAULT_LIMIT)
            .clamp(1, 20);

        let store = self.storage.memory_chunks_store();

        let count = store.count().await.unwrap_or(0);
        if count == 0 {
            return Ok(ToolOutput::success(
                "Memory index is empty — indexing runs in the background every 5 minutes. \
                 Try again shortly after the assistant has started."
                    .to_string(),
            ));
        }

        // Step 1: FTS5 with keyword expansion.
        let fts_query = build_fts_query(&query);
        let fts_hits = match store.search_fts(&fts_query, limit * 4).await {
            Ok(hits) => hits,
            Err(e) => return Ok(ToolOutput::error(format!("Search failed: {e}"))),
        };

        if fts_hits.is_empty() {
            return Ok(ToolOutput::success(format!(
                "No results found for query: {query}"
            )));
        }

        // Step 2: Embed query for hybrid reranking.
        let query_embedding = self.llm.embed(&query).await.ok();

        // Step 3: Build cosine map over FTS hits.
        let cos_map: HashMap<i64, f32> = if let Some(ref qvec) = query_embedding {
            let hit_ids: Vec<i64> = fts_hits.iter().map(|h| h.chunk_id).collect();
            store
                .get_embeddings_by_ids(&hit_ids)
                .await
                .unwrap_or_default()
                .iter()
                .filter_map(|chunk| {
                    chunk
                        .embedding
                        .as_ref()
                        .map(|evec| (chunk.id, cosine_similarity(qvec, evec)))
                })
                .collect()
        } else {
            HashMap::new()
        };

        // Step 4: Hybrid score + temporal decay → Candidate list.
        let today = SystemClock.now().with_timezone(&chrono::Local).date_naive();

        let min_rank = fts_hits
            .iter()
            .map(|h| h.rank)
            .fold(f64::INFINITY, f64::min);
        let max_rank = fts_hits
            .iter()
            .map(|h| h.rank)
            .fold(f64::NEG_INFINITY, f64::max);
        let rank_range = (max_rank - min_rank).max(1e-9);

        let mut candidates: Vec<Candidate> = fts_hits
            .iter()
            .map(|h| {
                let bm25_norm = 1.0 - (h.rank - min_rank) / rank_range;
                let cos = cos_map.get(&h.chunk_id).copied().unwrap_or(0.0) as f64;
                let raw_score = VECTOR_WEIGHT * cos + BM25_WEIGHT * bm25_norm;
                let decay = temporal_decay(&h.file_path, today);
                Candidate {
                    chunk_id: h.chunk_id,
                    file_path: h.file_path.clone(),
                    content: h.content.clone(),
                    score: raw_score * decay,
                    embedding: None,
                }
            })
            .collect();

        // Attach chunk embeddings needed for MMR diversity.
        if query_embedding.is_some() {
            let hit_ids: Vec<i64> = candidates.iter().map(|c| c.chunk_id).collect();
            let emb_map: HashMap<i64, Vec<f32>> = store
                .get_embeddings_by_ids(&hit_ids)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter_map(|c| c.embedding.map(|e| (c.id, e)))
                .collect();
            for c in &mut candidates {
                c.embedding = emb_map.get(&c.chunk_id).cloned();
            }
        }

        // Step 5: MMR re-ranking (when embeddings exist) or plain sort.
        let ranked: Vec<String> = if query_embedding.is_some() {
            mmr_select(candidates, limit as usize)
                .into_iter()
                .map(|c| {
                    format!(
                        "[{:.3}] {}\n{}",
                        c.score,
                        shorten_path(&c.file_path),
                        c.content.trim()
                    )
                })
                .collect()
        } else {
            candidates.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            candidates
                .into_iter()
                .take(limit as usize)
                .map(|c| format!("[fts] {}\n{}", shorten_path(&c.file_path), c.content.trim()))
                .collect()
        };

        let output = format!(
            "Memory search results for \"{query}\" ({} found):\n\n{}",
            ranked.len(),
            ranked.join("\n\n---\n\n")
        );

        Ok(ToolOutput::success(output))
    }
}

// ---------------------------------------------------------------------------
// Query expansion
// ---------------------------------------------------------------------------

/// Build an FTS5 query from a natural-language string.
///
/// Quotes the original phrase for exact-phrase matching, then appends
/// meaningful keywords with OR so conversational queries still surface hits.
///
/// Example: `"that thing we discussed about the API key"` →
///   `"that thing we discussed about the API key" OR "api" OR "key"`
fn build_fts_query(query: &str) -> String {
    let sanitised = query.replace('"', " ");
    let quoted_phrase = format!("\"{}\"", sanitised.trim());

    let stop = stop_words_for(query);
    let keywords: HashSet<String> = query
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .map(str::trim)
        .filter(|t| t.len() >= 3)
        .filter(|t| !stop.contains(&t.to_lowercase()))
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect();

    if keywords.is_empty() {
        return quoted_phrase;
    }

    let mut kw_vec: Vec<String> = keywords.into_iter().collect();
    kw_vec.sort(); // deterministic output for tests
    format!("{} OR {}", quoted_phrase, kw_vec.join(" OR "))
}

// ---------------------------------------------------------------------------
// Temporal decay
// ---------------------------------------------------------------------------

/// Compute the temporal decay multiplier for a chunk's file path.
///
/// Daily notes (`…/memory/YYYY-MM-DD.md`) decay with a 30-day half-life.
/// All other files (MEMORY.md, SOUL.md, etc.) return 1.0 (evergreen).
fn temporal_decay(file_path: &str, today: NaiveDate) -> f64 {
    let stem = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") else {
        return 1.0;
    };

    let age_days = (today - date).num_days().max(0) as f64;
    let lambda = std::f64::consts::LN_2 / DECAY_HALF_LIFE_DAYS;
    (-lambda * age_days).exp()
}

// ---------------------------------------------------------------------------
// MMR (Maximal Marginal Relevance)
// ---------------------------------------------------------------------------

/// Select up to `k` candidates using Maximal Marginal Relevance.
///
/// At each step picks the candidate maximising:
///   `MMR_LAMBDA × relevance − (1 − MMR_LAMBDA) × max_sim_to_selected`
fn mmr_select(mut candidates: Vec<Candidate>, k: usize) -> Vec<Candidate> {
    if candidates.is_empty() || k == 0 {
        return Vec::new();
    }

    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut selected: Vec<Candidate> = Vec::with_capacity(k);
    let mut remaining: Vec<Candidate> = candidates;

    while selected.len() < k && !remaining.is_empty() {
        let best_idx = remaining
            .iter()
            .enumerate()
            .map(|(i, cand)| {
                let max_sim = selected
                    .iter()
                    .map(|s| match (&cand.embedding, &s.embedding) {
                        (Some(a), Some(b)) => cosine_similarity(a, b) as f64,
                        _ => 0.0,
                    })
                    .fold(0.0_f64, f64::max);
                let mmr = MMR_LAMBDA * cand.score - (1.0 - MMR_LAMBDA) * max_sim;
                (i, mmr)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        selected.push(remaining.remove(best_idx));
    }

    selected
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let dot: f32 = a[..len]
        .iter()
        .zip(b[..len].iter())
        .map(|(x, y)| x * y)
        .sum();
    let norm_a: f32 = a[..len].iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b[..len].iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.rsplitn(3, '/').collect();
    match parts.len() {
        0 => path.to_string(),
        1 => parts[0].to_string(),
        2 => format!("{}/{}", parts[1], parts[0]),
        _ => format!("…/{}/{}", parts[1], parts[0]),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Query expansion --

    #[test]
    fn plain_keywords_expand() {
        let q = build_fts_query("API key");
        assert!(q.contains("\"API key\""), "missing quoted phrase: {q}");
        assert!(q.contains("OR"), "missing expansion: {q}");
    }

    #[test]
    fn conversational_query_expands_keywords() {
        let q = build_fts_query("that thing we discussed about the API key");
        assert!(q.contains("\"that thing we discussed about the API key\""));
        let lower = q.to_lowercase();
        assert!(
            lower.contains("\"api\"") || lower.contains("\"key\""),
            "expected keyword expansion: {q}"
        );
    }

    #[test]
    fn all_stop_words_returns_only_quoted_phrase() {
        let q = build_fts_query("the a an");
        assert!(!q.contains("OR"), "unexpected expansion: {q}");
    }

    #[test]
    fn empty_query_returns_quoted_empty() {
        let q = build_fts_query("");
        assert_eq!(q, "\"\"");
    }

    // -- Temporal decay --

    #[test]
    fn evergreen_file_has_decay_1() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 26).unwrap();
        assert_eq!(
            temporal_decay("/home/user/.assistant/MEMORY.md", today),
            1.0
        );
        assert_eq!(temporal_decay("/home/user/.assistant/SOUL.md", today), 1.0);
    }

    #[test]
    fn todays_note_has_decay_1() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 26).unwrap();
        let d = temporal_decay("/home/user/.assistant/memory/2026-03-26.md", today);
        assert!((d - 1.0).abs() < 1e-9, "expected ~1.0, got {d}");
    }

    #[test]
    fn note_at_half_life_has_decay_half() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 26).unwrap();
        // 30 days before 2026-03-26 = 2026-02-24
        let d = temporal_decay("/home/user/.assistant/memory/2026-02-24.md", today);
        assert!((d - 0.5).abs() < 0.01, "expected ~0.5, got {d}");
    }

    #[test]
    fn note_60_days_ago_has_decay_quarter() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 26).unwrap();
        // 60 days before 2026-03-26 = 2026-01-25
        let d = temporal_decay("/home/user/.assistant/memory/2026-01-25.md", today);
        assert!((d - 0.25).abs() < 0.02, "expected ~0.25, got {d}");
    }
}
