//! Builtin handler for the `memory-search` tool.
//!
//! Performs a hybrid FTS5 + cosine-similarity vector search over indexed
//! memory chunks.  If embedding is available, results are re-ranked using
//! a 50/50 blend of normalised FTS5 rank and cosine similarity.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_llm::LlmProvider;
use assistant_storage::StorageLayer;
use async_trait::async_trait;

const DEFAULT_LIMIT: i64 = 5;

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
                    "type": "number",
                    "description": "Maximum number of results to return (default: 5, max: 20)"
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

        // Check if there are any indexed chunks.
        let count = store.count().await.unwrap_or(0);
        if count == 0 {
            return Ok(ToolOutput::success(
                "Memory index is empty — indexing runs in the background every 5 minutes. \
                 Try again shortly after the assistant has started."
                    .to_string(),
            ));
        }

        // Step 1: FTS5 keyword search.
        let fts_query = escape_fts_query(&query);
        let fts_results = store.search_fts(&fts_query, limit * 2).await;

        let fts_hits = match fts_results {
            Ok(hits) => hits,
            Err(e) => {
                return Ok(ToolOutput::error(format!("Search failed: {e}")));
            }
        };

        if fts_hits.is_empty() {
            return Ok(ToolOutput::success(format!(
                "No results found for query: {query}"
            )));
        }

        // Step 2: Try to embed the query for hybrid reranking.
        let query_embedding = self.llm.embed(&query).await.ok();

        // Step 3: Score and rank results.
        let ranked = if let Some(qvec) = query_embedding {
            let hit_ids: Vec<i64> = fts_hits.iter().map(|h| h.chunk_id).collect();
            let embedded = store
                .get_embeddings_by_ids(&hit_ids)
                .await
                .unwrap_or_default();

            let mut cos_map: HashMap<i64, f32> = HashMap::new();
            for chunk in &embedded {
                if let Some(evec) = &chunk.embedding {
                    let sim = cosine_similarity(&qvec, evec);
                    cos_map.insert(chunk.id, sim);
                }
            }

            let min_rank = fts_hits
                .iter()
                .map(|h| h.rank)
                .fold(f64::INFINITY, f64::min);
            let max_rank = fts_hits
                .iter()
                .map(|h| h.rank)
                .fold(f64::NEG_INFINITY, f64::max);
            let rank_range = (max_rank - min_rank).max(1e-9);

            let mut scored: Vec<(f64, &assistant_storage::FtsMatch)> = fts_hits
                .iter()
                .map(|h| {
                    let fts_norm = 1.0 - (h.rank - min_rank) / rank_range;
                    let cos = cos_map.get(&h.chunk_id).copied().unwrap_or(0.0) as f64;
                    let hybrid = 0.5 * fts_norm + 0.5 * cos;
                    (hybrid, h)
                })
                .collect();

            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored
                .into_iter()
                .take(limit as usize)
                .map(|(score, h)| {
                    format!(
                        "[{:.3}] {}\n{}",
                        score,
                        shorten_path(&h.file_path),
                        h.content.trim()
                    )
                })
                .collect::<Vec<_>>()
        } else {
            fts_hits
                .into_iter()
                .take(limit as usize)
                .map(|h| format!("[fts] {}\n{}", shorten_path(&h.file_path), h.content.trim()))
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
// Helpers
// ---------------------------------------------------------------------------

/// Cosine similarity between two f32 vectors.
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

/// Escape special FTS5 query characters.
fn escape_fts_query(query: &str) -> String {
    let sanitised = query.replace('"', " ");
    format!("\"{}\"", sanitised)
}

/// Shorten an absolute path for display (show only the last 2 components).
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.rsplitn(3, '/').collect();
    match parts.len() {
        0 => path.to_string(),
        1 => parts[0].to_string(),
        2 => format!("{}/{}", parts[1], parts[0]),
        _ => format!("…/{}/{}", parts[1], parts[0]),
    }
}
