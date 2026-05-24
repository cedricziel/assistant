//! Storage layer for memory chunks — used by the background memory indexer.
//!
//! Each memory file is split into chunks of ~400 characters and stored in
//! `memory_chunks` alongside an optional embedding vector (BLOB of raw f32
//! bytes).  A companion FTS5 virtual table enables fast full-text search.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

/// A single stored chunk from a memory file.
pub struct StoredChunk {
    pub id: i64,
    pub file_path: String,
    pub chunk_index: i32,
    pub content: String,
    /// Dense embedding vector.  `None` when not yet embedded.
    pub embedding: Option<Vec<f32>>,
    pub file_hash: String,
}

/// A full-text search match.
pub struct FtsMatch {
    pub chunk_id: i64,
    pub file_path: String,
    pub content: String,
    /// FTS5 rank score (lower is better — negate for descending sort).
    pub rank: f64,
}

// Row type alias to avoid clippy::type_complexity lint.
type ChunkRow = (i64, String, i32, String, Option<Vec<u8>>, String);

/// Trait-based interface for memory chunk persistence.
#[async_trait]
pub trait MemoryChunkStore: Send + Sync {
    async fn get_file_hash(&self, file_path: &str) -> Result<Option<String>>;
    async fn delete_file_chunks(&self, file_path: &str) -> Result<()>;
    async fn upsert_chunk(
        &self,
        file_path: &str,
        file_hash: &str,
        chunk_index: i32,
        content: &str,
    ) -> Result<i64>;
    async fn update_embedding(&self, id: i64, embedding: &[f32]) -> Result<()>;
    async fn get_unembedded(&self, limit: i64) -> Result<Vec<StoredChunk>>;
    async fn get_all_embedded(&self) -> Result<Vec<StoredChunk>>;
    async fn get_embeddings_by_ids(&self, ids: &[i64]) -> Result<Vec<StoredChunk>>;
    async fn search_fts(&self, query: &str, limit: i64) -> Result<Vec<FtsMatch>>;
    async fn replace_file_chunks(
        &self,
        file_path: &str,
        file_hash: &str,
        chunks: &[String],
    ) -> Result<()>;
    async fn count(&self) -> Result<i64>;
}

/// SQLite-backed store for memory chunks and their embeddings.
pub struct SqliteMemoryChunkStore(pub(crate) SqlitePool);

impl SqliteMemoryChunkStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self(pool)
    }
}

#[async_trait]
impl MemoryChunkStore for SqliteMemoryChunkStore {
    /// Return the stored SHA-256 hash for `file_path`, if any chunks exist.
    async fn get_file_hash(&self, file_path: &str) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT file_hash FROM memory_chunks WHERE file_path = ? LIMIT 1")
                .bind(file_path)
                .fetch_optional(&self.0)
                .await?;
        Ok(row.map(|(h,)| h))
    }

    /// Delete all chunks for `file_path` (call before re-indexing a changed file).
    async fn delete_file_chunks(&self, file_path: &str) -> Result<()> {
        sqlx::query("DELETE FROM memory_chunks WHERE file_path = ?")
            .bind(file_path)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    /// Insert or replace a single chunk, returning its row id.
    async fn upsert_chunk(
        &self,
        file_path: &str,
        file_hash: &str,
        chunk_index: i32,
        content: &str,
    ) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO memory_chunks (file_path, file_hash, chunk_index, content)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(file_path, chunk_index) DO UPDATE SET
               content    = excluded.content,
               file_hash  = excluded.file_hash,
               embedding  = NULL,
               indexed_at = CURRENT_TIMESTAMP
             RETURNING id",
        )
        .bind(file_path)
        .bind(file_hash)
        .bind(chunk_index)
        .bind(content)
        .fetch_one(&self.0)
        .await?;
        Ok(id)
    }

    /// Persist an embedding vector for the given chunk id.
    ///
    /// The vector is stored as raw little-endian f32 bytes.
    async fn update_embedding(&self, id: i64, embedding: &[f32]) -> Result<()> {
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        sqlx::query("UPDATE memory_chunks SET embedding = ? WHERE id = ?")
            .bind(bytes)
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    /// Return up to `limit` chunks that have no embedding yet.
    async fn get_unembedded(&self, limit: i64) -> Result<Vec<StoredChunk>> {
        let rows: Vec<ChunkRow> = sqlx::query_as(
            "SELECT id, file_path, chunk_index, content, embedding, file_hash
             FROM memory_chunks
             WHERE embedding IS NULL
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.0)
        .await?;

        Ok(rows.into_iter().map(decode_chunk).collect())
    }

    /// Return all chunks that have an embedding (for cosine similarity search).
    async fn get_all_embedded(&self) -> Result<Vec<StoredChunk>> {
        let rows: Vec<ChunkRow> = sqlx::query_as(
            "SELECT id, file_path, chunk_index, content, embedding, file_hash
             FROM memory_chunks
             WHERE embedding IS NOT NULL",
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rows.into_iter().map(decode_chunk).collect())
    }

    /// Return embedded chunks whose IDs are in `ids`.
    ///
    /// More efficient than [`get_all_embedded`] when you only need the embeddings
    /// for a known set of FTS hits.
    async fn get_embeddings_by_ids(&self, ids: &[i64]) -> Result<Vec<StoredChunk>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // Build `... WHERE id IN (?, ?, …)` with one bound parameter per id.
        // `QueryBuilder::push_bind` emits the placeholders, so no id value is
        // ever interpolated into the SQL string.
        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "SELECT id, file_path, chunk_index, content, embedding, file_hash \
             FROM memory_chunks WHERE id IN (",
        );
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(*id);
        }
        separated.push_unseparated(") AND embedding IS NOT NULL");

        let rows = builder
            .build_query_as::<ChunkRow>()
            .fetch_all(&self.0)
            .await?;
        Ok(rows.into_iter().map(decode_chunk).collect())
    }

    /// Full-text search using the FTS5 virtual table.
    ///
    /// Results are ranked by FTS5's built-in BM25 rank (lower rank = better).
    async fn search_fts(&self, query: &str, limit: i64) -> Result<Vec<FtsMatch>> {
        let rows: Vec<(i64, String, String, f64)> = sqlx::query_as(
            "SELECT mc.id, mc.file_path, mc.content, fts.rank
             FROM memory_chunks_fts fts
             JOIN memory_chunks mc ON mc.id = fts.rowid
             WHERE memory_chunks_fts MATCH ?
             ORDER BY fts.rank
             LIMIT ?",
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.0)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(chunk_id, file_path, content, rank)| FtsMatch {
                chunk_id,
                file_path,
                content,
                rank,
            })
            .collect())
    }

    /// Atomically replace all chunks for a file in a single transaction.
    ///
    /// Deletes existing chunks and inserts the new ones so that a failure
    /// mid-way leaves the old data intact rather than a partial update.
    async fn replace_file_chunks(
        &self,
        file_path: &str,
        file_hash: &str,
        chunks: &[String],
    ) -> Result<()> {
        let mut tx = self.0.begin().await?;

        sqlx::query("DELETE FROM memory_chunks WHERE file_path = ?")
            .bind(file_path)
            .execute(&mut *tx)
            .await?;

        if chunks.is_empty() {
            // Persist a hash-only sentinel so `get_file_hash` still returns
            // the hash on the next cycle and the file is not re-indexed.
            sqlx::query(
                "INSERT INTO memory_chunks (file_path, file_hash, chunk_index, content)
                 VALUES (?, ?, -1, '')",
            )
            .bind(file_path)
            .bind(file_hash)
            .execute(&mut *tx)
            .await?;
        } else {
            for (idx, content) in chunks.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO memory_chunks (file_path, file_hash, chunk_index, content)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(file_path)
                .bind(file_hash)
                .bind(idx as i32)
                .bind(content)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Return the total number of indexed chunks.
    async fn count(&self) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_chunks")
            .fetch_one(&self.0)
            .await?;
        Ok(n)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn decode_chunk(row: ChunkRow) -> StoredChunk {
    let (id, file_path, chunk_index, content, embedding_bytes, file_hash) = row;
    let embedding = embedding_bytes.map(|b| {
        b.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    });
    StoredChunk {
        id,
        file_path,
        chunk_index,
        content,
        embedding,
        file_hash,
    }
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

#[derive(Default)]
struct MemoryChunkMemoryState {
    next_id: i64,
    chunks: HashMap<i64, StoredChunk>,
}

/// In-memory [`MemoryChunkStore`]. HashMap-backed; FTS search does a naive
/// substring scan. Use for test seams; production embedding indexers
/// should run against SQLite.
pub struct InMemoryMemoryChunkStore {
    state: Arc<Mutex<MemoryChunkMemoryState>>,
}

impl InMemoryMemoryChunkStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryChunkMemoryState::default())),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, MemoryChunkMemoryState> {
        match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

impl Default for InMemoryMemoryChunkStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryChunkStore for InMemoryMemoryChunkStore {
    async fn get_file_hash(&self, file_path: &str) -> Result<Option<String>> {
        let state = self.lock();
        Ok(state
            .chunks
            .values()
            .find(|c| c.file_path == file_path)
            .map(|c| c.file_hash.clone()))
    }

    async fn delete_file_chunks(&self, file_path: &str) -> Result<()> {
        let mut state = self.lock();
        state.chunks.retain(|_, c| c.file_path != file_path);
        Ok(())
    }

    async fn upsert_chunk(
        &self,
        file_path: &str,
        file_hash: &str,
        chunk_index: i32,
        content: &str,
    ) -> Result<i64> {
        let mut state = self.lock();
        // Search for existing chunk at this (file_path, chunk_index).
        let existing_id = state
            .chunks
            .iter()
            .find(|(_, c)| c.file_path == file_path && c.chunk_index == chunk_index)
            .map(|(id, _)| *id);
        let id = if let Some(id) = existing_id {
            if let Some(c) = state.chunks.get_mut(&id) {
                c.content = content.to_string();
                c.file_hash = file_hash.to_string();
                c.embedding = None;
            }
            id
        } else {
            state.next_id += 1;
            let id = state.next_id;
            state.chunks.insert(
                id,
                StoredChunk {
                    id,
                    file_path: file_path.to_string(),
                    chunk_index,
                    content: content.to_string(),
                    embedding: None,
                    file_hash: file_hash.to_string(),
                },
            );
            id
        };
        Ok(id)
    }

    async fn update_embedding(&self, id: i64, embedding: &[f32]) -> Result<()> {
        let mut state = self.lock();
        if let Some(c) = state.chunks.get_mut(&id) {
            c.embedding = Some(embedding.to_vec());
        }
        Ok(())
    }

    async fn get_unembedded(&self, limit: i64) -> Result<Vec<StoredChunk>> {
        let state = self.lock();
        let mut out: Vec<StoredChunk> = state
            .chunks
            .values()
            .filter(|c| c.embedding.is_none())
            .map(|c| StoredChunk {
                id: c.id,
                file_path: c.file_path.clone(),
                chunk_index: c.chunk_index,
                content: c.content.clone(),
                embedding: None,
                file_hash: c.file_hash.clone(),
            })
            .collect();
        out.sort_by_key(|c| c.id);
        if limit > 0 {
            out.truncate(limit as usize);
        }
        Ok(out)
    }

    async fn get_all_embedded(&self) -> Result<Vec<StoredChunk>> {
        let state = self.lock();
        let mut out: Vec<StoredChunk> = state
            .chunks
            .values()
            .filter(|c| c.embedding.is_some())
            .map(|c| StoredChunk {
                id: c.id,
                file_path: c.file_path.clone(),
                chunk_index: c.chunk_index,
                content: c.content.clone(),
                embedding: c.embedding.clone(),
                file_hash: c.file_hash.clone(),
            })
            .collect();
        out.sort_by_key(|c| c.id);
        Ok(out)
    }

    async fn get_embeddings_by_ids(&self, ids: &[i64]) -> Result<Vec<StoredChunk>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let state = self.lock();
        let id_set: std::collections::HashSet<i64> = ids.iter().copied().collect();
        let out: Vec<StoredChunk> = state
            .chunks
            .values()
            .filter(|c| id_set.contains(&c.id) && c.embedding.is_some())
            .map(|c| StoredChunk {
                id: c.id,
                file_path: c.file_path.clone(),
                chunk_index: c.chunk_index,
                content: c.content.clone(),
                embedding: c.embedding.clone(),
                file_hash: c.file_hash.clone(),
            })
            .collect();
        Ok(out)
    }

    async fn search_fts(&self, query: &str, limit: i64) -> Result<Vec<FtsMatch>> {
        // Naive substring scan — no real FTS5 ranking in-memory.
        let q = query.to_lowercase();
        let state = self.lock();
        let mut out: Vec<FtsMatch> = state
            .chunks
            .values()
            .filter(|c| c.content.to_lowercase().contains(&q))
            .map(|c| FtsMatch {
                chunk_id: c.id,
                file_path: c.file_path.clone(),
                content: c.content.clone(),
                rank: 0.0,
            })
            .collect();
        if limit > 0 {
            out.truncate(limit as usize);
        }
        Ok(out)
    }

    async fn replace_file_chunks(
        &self,
        file_path: &str,
        file_hash: &str,
        chunks: &[String],
    ) -> Result<()> {
        let mut state = self.lock();
        state.chunks.retain(|_, c| c.file_path != file_path);
        for (i, content) in chunks.iter().enumerate() {
            state.next_id += 1;
            let id = state.next_id;
            state.chunks.insert(
                id,
                StoredChunk {
                    id,
                    file_path: file_path.to_string(),
                    chunk_index: i as i32,
                    content: content.clone(),
                    embedding: None,
                    file_hash: file_hash.to_string(),
                },
            );
        }
        Ok(())
    }

    async fn count(&self) -> Result<i64> {
        let state = self.lock();
        Ok(state.chunks.len() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    /// Seed three chunks, embed only the first and third, and confirm
    /// `get_embeddings_by_ids` returns exactly the requested *embedded* rows.
    /// Guards the dynamic `IN (?, ?, …)` clause and the `IS NOT NULL` filter.
    #[tokio::test]
    async fn get_embeddings_by_ids_returns_only_embedded_matches() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = SqliteMemoryChunkStore::new(storage.pool.clone());

        let id0 = store
            .upsert_chunk("notes.md", "hash", 0, "first")
            .await
            .unwrap();
        let id1 = store
            .upsert_chunk("notes.md", "hash", 1, "second")
            .await
            .unwrap();
        let id2 = store
            .upsert_chunk("notes.md", "hash", 2, "third")
            .await
            .unwrap();

        store.update_embedding(id0, &[1.0, 2.0, 3.0]).await.unwrap();
        store.update_embedding(id2, &[7.0, 8.0]).await.unwrap();

        // Request all three; only id0 and id2 are embedded.
        let mut found = store.get_embeddings_by_ids(&[id0, id1, id2]).await.unwrap();
        found.sort_by_key(|c| c.id);

        assert_eq!(found.len(), 2, "only embedded rows should be returned");
        assert_eq!(found[0].id, id0);
        assert_eq!(found[0].embedding.as_deref(), Some(&[1.0, 2.0, 3.0][..]));
        assert_eq!(found[1].id, id2);
        assert_eq!(found[1].embedding.as_deref(), Some(&[7.0, 8.0][..]));
    }

    #[tokio::test]
    async fn get_embeddings_by_ids_empty_input_returns_empty() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = SqliteMemoryChunkStore::new(storage.pool.clone());

        let found = store.get_embeddings_by_ids(&[]).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn get_embeddings_by_ids_skips_unembedded_ids() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = SqliteMemoryChunkStore::new(storage.pool.clone());

        let id = store
            .upsert_chunk("notes.md", "hash", 0, "unembedded")
            .await
            .unwrap();

        let found = store.get_embeddings_by_ids(&[id]).await.unwrap();
        assert!(found.is_empty(), "unembedded chunk must be excluded");
    }
}
