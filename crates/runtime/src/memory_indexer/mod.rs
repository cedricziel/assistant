//! Background memory indexer — chunks memory files and embeds them in SQLite.
//!
//! `MemoryIndexer` scans the assistant's memory files (SOUL.md, IDENTITY.md,
//! USER.md, MEMORY.md and daily notes) and maintains the `memory_chunks`
//! table.  Files whose SHA-256 hash hasn't changed since the last index run
//! are skipped.  After chunking, unembedded chunks are submitted to the LLM
//! provider's `embed()` endpoint.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{base_dir, resolve_dir, resolve_path, AssistantConfig};
use assistant_llm::LlmProvider;
use assistant_storage::{MemoryChunkStore, StorageLayer};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

/// Maximum bytes per text chunk.
const MAX_CHUNK_BYTES: usize = 400;
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

        // Scan the notes directory and filter all paths in the blocking context
        // so that no synchronous filesystem calls (read_dir, exists) run on the
        // async thread.
        let notes_dir = resolve_dir(&mem.notes_dir, &base, "memory");
        tokio::task::spawn_blocking(move || {
            let mut notes = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&notes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        notes.push(path);
                    }
                }
            }
            // Combine core files + notes and filter existence in one blocking pass.
            core_files
                .into_iter()
                .chain(notes)
                .filter(|p| p.exists())
                .collect()
        })
        .await
        .map_err(|e| {
            warn!(error = %e, "Memory file scan task failed");
            e
        })
        .unwrap_or_default()
    }

    /// Index a single file: compute hash, skip if unchanged, else re-chunk.
    async fn index_file(&self, store: &MemoryChunkStore, path: &Path) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let hash = sha256_hex(&content);
        let path_str = path.to_string_lossy().to_string();

        // Skip if hash unchanged.
        if store.get_file_hash(&path_str).await? == Some(hash.clone()) {
            debug!(file = %path.display(), "Memory file unchanged, skipping");
            return Ok(());
        }

        let chunks: Vec<String> = chunk_text(&content).collect();
        debug!(file = %path.display(), chunks = chunks.len(), "Indexing memory file");
        store.replace_file_chunks(&path_str, &hash, &chunks).await?;

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
                    let msg = e.to_string();
                    if msg.contains("not supported") || msg.contains("Not supported") {
                        debug!("Embedding unsupported by provider; skipping batch");
                        break;
                    }
                    debug!(chunk_id = chunk.id, error = %e, "Embedding skipped");
                }
            }
        }
        Ok(())
    }
}

/// Spawn a background task that re-indexes memory files every `interval`.
///
/// Returns a [`tokio::task::JoinHandle`] so the caller can abort or await the
/// task during graceful shutdown.
pub fn spawn_memory_indexer(
    config: &assistant_core::MemoryConfig,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
) -> tokio::task::JoinHandle<()> {
    let secs = config.indexing_interval_seconds.unwrap_or(300).max(1);
    let interval = std::time::Duration::from_secs(secs);
    let enabled = config.enabled;

    // Create a minimal AssistantConfig with just the memory section
    // for the MemoryIndexer
    let assistant_config = Arc::new(AssistantConfig {
        memory: config.clone(),
        ..AssistantConfig::default()
    });

    let indexer = Arc::new(MemoryIndexer::new(assistant_config, storage, llm));

    tokio::spawn(async move {
        if !enabled {
            info!("Memory indexer disabled, not starting");
            return;
        }

        info!("Memory indexer started (interval: {:?})", interval);

        // Run initial indexing
        if let Err(e) = indexer.index_all().await {
            warn!("Initial memory indexing failed: {}", e);
        }

        loop {
            tokio::time::sleep(interval).await;

            if let Err(e) = indexer.index_all().await {
                warn!("Memory indexing failed: {}", e);
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Text chunking
// ---------------------------------------------------------------------------

/// Split `text` into chunks of at most `MAX_CHUNK_BYTES` bytes.
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

        if para.len() <= MAX_CHUNK_BYTES {
            // Merge tiny fragment into the last chunk if it fits.
            if para.len() < 50 {
                if let Some(last) = chunks.last_mut() {
                    if last.len() + 1 + para.len() <= MAX_CHUNK_BYTES {
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
                if candidate.len() <= MAX_CHUNK_BYTES {
                    current = candidate;
                } else {
                    if !current.is_empty() {
                        chunks.push(current.clone());
                    }
                    // If a single sentence is too long, split by characters.
                    let mut start = 0;
                    while start < sentence.len() {
                        let end = (start + MAX_CHUNK_BYTES).min(sentence.len());
                        // Walk back to char boundary.
                        let mut end = end;
                        while end > start && !sentence.is_char_boundary(end) {
                            end -= 1;
                        }
                        chunks.push(sentence[start..end].to_string());
                        start = end;
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
            assert!(c.len() <= MAX_CHUNK_BYTES, "chunk too long: {}", c.len());
        }
    }

    #[test]
    fn chunk_text_long_paragraph() {
        let long = "word ".repeat(200);
        let chunks: Vec<_> = chunk_text(&long).collect();
        for c in &chunks {
            assert!(c.len() <= MAX_CHUNK_BYTES, "chunk too long: {}", c.len());
        }
    }

    #[test]
    fn sha256_is_stable() {
        let h1 = sha256_hex("hello");
        let h2 = sha256_hex("hello");
        assert_eq!(h1, h2);
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }

    #[test]
    fn chunk_text_empty_input() {
        // Empty string and whitespace-only double-newlines must yield no chunks.
        assert!(chunk_text("").collect::<Vec<_>>().is_empty());
        assert!(chunk_text("\n\n").collect::<Vec<_>>().is_empty());
        assert!(chunk_text("\n\n\n\n").collect::<Vec<_>>().is_empty());
    }

    #[test]
    fn chunk_text_tiny_fragments_are_merged() {
        // Build input made entirely of short (<50 char) segments separated by
        // double newlines.  All segments should be merged into larger chunks
        // (not one chunk per segment) and each chunk must stay within bounds.
        let segments: Vec<String> = (0..20).map(|i| format!("seg{i}")).collect();
        let text = segments.join("\n\n");
        let chunks: Vec<_> = chunk_text(&text).collect();
        // With 20 segments of ~5 bytes each, they should be merged into far
        // fewer chunks than 20.
        assert!(
            chunks.len() < segments.len(),
            "tiny fragments should be merged; got {} chunks for {} segments",
            chunks.len(),
            segments.len()
        );
        for c in &chunks {
            assert!(c.len() <= MAX_CHUNK_BYTES, "chunk too long: {}", c.len());
        }
    }

    #[test]
    fn chunk_text_multibyte_utf8() {
        // Emoji and non-Latin characters — verify no panics and byte bounds hold.
        let emoji_para = "🦀".repeat(80); // 80 × 4 bytes = 320 bytes, fits in one chunk
        let long_emoji = "🦀".repeat(200); // 800 bytes — must be split
        let text = format!("{emoji_para}\n\n{long_emoji}");
        let chunks: Vec<_> = chunk_text(&text).collect();
        assert!(!chunks.is_empty());
        for c in &chunks {
            assert!(
                c.len() <= MAX_CHUNK_BYTES,
                "chunk too long ({} bytes): starts with {:?}",
                c.len(),
                &c[..c.len().min(20)]
            );
        }
    }

    #[test]
    fn sha256_stable_on_multibyte() {
        let input = "日本語テスト🦀";
        let h1 = sha256_hex(input);
        let h2 = sha256_hex(input);
        assert_eq!(h1, h2);
        assert_ne!(sha256_hex(input), sha256_hex("different"));
    }
}
