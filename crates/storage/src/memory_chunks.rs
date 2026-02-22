//! Storage layer for memory chunks — used by the background memory indexer.
//!
//! Each memory file is split into chunks of ~400 characters and stored in
//! `memory_chunks` alongside an optional embedding vector (BLOB of raw f32
//! bytes).  A companion FTS5 virtual table enables fast full-text search.

use anyhow::Result;
use sqlx::SqlitePool;

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

/// SQLite-backed store for memory chunks and their embeddings.
pub struct MemoryChunkStore(pub(crate) SqlitePool);

impl MemoryChunkStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self(pool)
    }

    /// Return the stored SHA-256 hash for `file_path`, if any chunks exist.
    pub async fn get_file_hash(&self, file_path: &str) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT file_hash FROM memory_chunks WHERE file_path = ? LIMIT 1")
                .bind(file_path)
                .fetch_optional(&self.0)
                .await?;
        Ok(row.map(|(h,)| h))
    }

    /// Delete all chunks for `file_path` (call before re-indexing a changed file).
    pub async fn delete_file_chunks(&self, file_path: &str) -> Result<()> {
        sqlx::query("DELETE FROM memory_chunks WHERE file_path = ?")
            .bind(file_path)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    /// Insert or replace a single chunk, returning its row id.
    pub async fn upsert_chunk(
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
    pub async fn update_embedding(&self, id: i64, embedding: &[f32]) -> Result<()> {
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        sqlx::query("UPDATE memory_chunks SET embedding = ? WHERE id = ?")
            .bind(bytes)
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    /// Return up to `limit` chunks that have no embedding yet.
    pub async fn get_unembedded(&self, limit: i64) -> Result<Vec<StoredChunk>> {
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
    pub async fn get_all_embedded(&self) -> Result<Vec<StoredChunk>> {
        let rows: Vec<ChunkRow> = sqlx::query_as(
            "SELECT id, file_path, chunk_index, content, embedding, file_hash
             FROM memory_chunks
             WHERE embedding IS NOT NULL",
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rows.into_iter().map(decode_chunk).collect())
    }

    /// Full-text search using the FTS5 virtual table.
    ///
    /// Results are ranked by FTS5's built-in BM25 rank (lower rank = better).
    pub async fn search_fts(&self, query: &str, limit: i64) -> Result<Vec<FtsMatch>> {
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

    /// Return the total number of indexed chunks.
    pub async fn count(&self) -> Result<i64> {
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
