//! Background memory indexer — chunks memory files and embeds them in SQLite.
//!
//! `MemoryIndexer` scans the assistant's memory files (SOUL.md, IDENTITY.md,
//! USER.md, MEMORY.md and daily notes) and maintains the `memory_chunks`
//! table.  Files whose SHA-256 hash hasn't changed since the last index run
//! are skipped.  After chunking, unembedded chunks are submitted to the LLM
//! provider's `embed()` endpoint.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{base_dir, resolve_dir, resolve_path, AssistantConfig};
use assistant_llm::LlmProvider;
use assistant_storage::{MemoryChunkStore, StorageLayer};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

/// Maximum characters per text chunk.
const MAX_CHUNK_CHARS: usize = 400;
/// Maximum chunks to embed per indexing run (avoids long blocking on first run).
const EMBED_BATCH_SIZE: i64 = 100;

/// Indexes memory files into SQLite for full-text and vector search.
pub struct MemoryIndexer {
    config: Arc<AssistantConfig>,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
}

impl MemoryIndexer {
    pub fn new(
        config: Arc<AssistantConfig>,
        storage: Arc<StorageLayer>,
        llm: Arc<dyn LlmProvider>,
    ) -> Self {
        Self {
            config,
            storage,
            llm,
        }
    }

    /// Index all memory files, skipping unchanged ones.
    ///
    /// This method is designed to be called from a background task.
    pub async fn index_all(&self) -> Result<()> {
        let store = self.storage.memory_chunks_store();
        for file in self.memory_files().await {
            if let Err(e) = self.index_file(&store, &file).await {
                warn!(file = %file.display(), error = %e, "Failed to index memory file");
            }
        }
        self.embed_pending(&store).await
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Collect all memory file paths that should be indexed.
    ///
    /// Uses `spawn_blocking` so that directory scanning does not block the
    /// tokio worker thread.
    async fn memory_files(&self) -> Vec<PathBuf> {
        let mem = self.config.memory.clone();
        let base = base_dir();

        let core_files = vec![
            resolve_path(&mem.soul_path, &base, "SOUL.md"),
            resolve_path(&mem.identity_path, &base, "IDENTITY.md"),
            resolve_path(&mem.user_path, &base, "USER.md"),
            resolve_path(&mem.memory_path, &base, "MEMORY.md"),
        ];

        // Add daily notes from the notes directory (blocking read_dir wrapped).
        let notes_dir = resolve_dir(&mem.notes_dir, &base, "memory");
        let note_files = tokio::task::spawn_blocking(move || {
            let mut notes = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&notes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        notes.push(path);
                    }
                }
            }
            notes
        })
        .await
        .unwrap_or_default();

        // Only include files that actually exist.
        core_files
            .into_iter()
            .chain(note_files)
            .filter(|p| p.exists())
            .collect()
    }

    /// Index a single file: compute hash, skip if unchanged, else re-chunk.
    async fn index_file(&self, store: &MemoryChunkStore, path: &PathBuf) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let hash = sha256_hex(&content);
        let path_str = path.to_string_lossy().to_string();

        // Skip if hash unchanged.
        if store.get_file_hash(&path_str).await? == Some(hash.clone()) {
            debug!(file = %path.display(), "Memory file unchanged, skipping");
            return Ok(());
        }

        debug!(file = %path.display(), "Indexing memory file");
        store.delete_file_chunks(&path_str).await?;

        for (idx, chunk) in chunk_text(&content).enumerate() {
            store
                .upsert_chunk(&path_str, &hash, idx as i32, &chunk)
                .await?;
        }

        Ok(())
    }

    /// Embed all unembedded chunks up to `EMBED_BATCH_SIZE`.
    async fn embed_pending(&self, store: &MemoryChunkStore) -> Result<()> {
        let unembedded = store.get_unembedded(EMBED_BATCH_SIZE).await?;
        for chunk in unembedded {
            match self.llm.embed(&chunk.content).await {
                Ok(vec) => {
                    if let Err(e) = store.update_embedding(chunk.id, &vec).await {
                        warn!(chunk_id = chunk.id, error = %e, "Failed to store embedding");
                    }
                }
                Err(e) => {
                    // Embedding is optional — log and continue.
                    debug!(chunk_id = chunk.id, error = %e, "Embedding skipped");
                }
            }
        }
        Ok(())
    }
}

/// Spawn a background task that re-indexes memory files every `interval`.
pub fn spawn_memory_indexer(indexer: Arc<MemoryIndexer>, interval: std::time::Duration) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = indexer.index_all().await {
                warn!("Memory indexing failed: {e}");
            }
            tokio::time::sleep(interval).await;
        }
    });
}

// ---------------------------------------------------------------------------
// Text chunking
// ---------------------------------------------------------------------------

/// Split `text` into chunks of at most `MAX_CHUNK_CHARS` characters.
///
/// Strategy: split on double-newline (paragraph boundaries), then further
/// split large paragraphs, and merge tiny fragments into the previous chunk.
fn chunk_text(text: &str) -> impl Iterator<Item = String> {
    let mut chunks: Vec<String> = Vec::new();

    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }

        if para.len() <= MAX_CHUNK_CHARS {
            // Merge tiny fragment into the last chunk if it fits.
            if para.len() < 50 {
                if let Some(last) = chunks.last_mut() {
                    if last.len() + 1 + para.len() <= MAX_CHUNK_CHARS {
                        last.push('\n');
                        last.push_str(para);
                        continue;
                    }
                }
            }
            chunks.push(para.to_string());
        } else {
            // Split large paragraph into smaller pieces at sentence boundaries.
            let mut current = String::new();
            for sentence in para.split(". ") {
                let sentence = sentence.trim();
                if sentence.is_empty() {
                    continue;
                }
                let candidate = if current.is_empty() {
                    sentence.to_string()
                } else {
                    format!("{}. {}", current, sentence)
                };
                if candidate.len() <= MAX_CHUNK_CHARS {
                    current = candidate;
                } else {
                    if !current.is_empty() {
                        chunks.push(current.clone());
                    }
                    // If a single sentence is too long, split by characters.
                    let mut start = 0;
                    let bytes = sentence.as_bytes();
                    while start < sentence.len() {
                        let end = (start + MAX_CHUNK_CHARS).min(sentence.len());
                        // Walk back to char boundary.
                        let mut end = end;
                        while end > start && !sentence.is_char_boundary(end) {
                            end -= 1;
                        }
                        chunks.push(sentence[start..end].to_string());
                        start = end;
                        let _ = bytes; // suppress unused warning
                    }
                    current = String::new();
                }
            }
            if !current.is_empty() {
                chunks.push(current);
            }
        }
    }

    chunks.into_iter()
}

// ---------------------------------------------------------------------------
// SHA-256 helper
// ---------------------------------------------------------------------------

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_text_basic() {
        let text = "Hello world.\n\nThis is a second paragraph.\n\nShort.";
        let chunks: Vec<_> = chunk_text(text).collect();
        assert!(!chunks.is_empty());
        for c in &chunks {
            assert!(c.len() <= MAX_CHUNK_CHARS, "chunk too long: {}", c.len());
        }
    }

    #[test]
    fn chunk_text_long_paragraph() {
        let long = "word ".repeat(200);
        let chunks: Vec<_> = chunk_text(&long).collect();
        for c in &chunks {
            assert!(c.len() <= MAX_CHUNK_CHARS, "chunk too long: {}", c.len());
        }
    }

    #[test]
    fn sha256_is_stable() {
        let h1 = sha256_hex("hello");
        let h2 = sha256_hex("hello");
        assert_eq!(h1, h2);
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }
}
