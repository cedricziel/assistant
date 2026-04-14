//! In-memory store for tool-synthesized audio blobs.
//!
//! Audio entries expire after [`TTL`] and are swept periodically.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use uuid::Uuid;

/// How long synthesized audio is kept in memory.
const TTL: Duration = Duration::from_secs(3600); // 1 hour

/// Inner map type alias to keep the field declaration readable.
type AudioMap = HashMap<Uuid, (Vec<u8>, Instant)>;

/// Shared in-memory store for audio blobs produced by the `voice-response` tool.
#[derive(Clone, Default)]
pub struct AudioStore {
    inner: Arc<RwLock<AudioMap>>,
}

impl AudioStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store audio bytes under a fresh UUID. Returns the UUID.
    pub async fn insert(&self, audio: Vec<u8>) -> Uuid {
        let id = Uuid::new_v4();
        self.inner.write().await.insert(id, (audio, Instant::now()));
        id
    }

    /// Retrieve audio bytes by ID. Returns `None` if not found or expired.
    pub async fn get(&self, id: Uuid) -> Option<Vec<u8>> {
        let map = self.inner.read().await;
        map.get(&id).and_then(|(data, inserted)| {
            if inserted.elapsed() < TTL {
                Some(data.clone())
            } else {
                None
            }
        })
    }

    /// Remove all expired entries.
    pub async fn sweep(&self) {
        let mut map = self.inner.write().await;
        map.retain(|_, (_, inserted)| inserted.elapsed() < TTL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_get_roundtrip() {
        let store = AudioStore::new();
        let audio = b"fake-mp3-bytes".to_vec();

        let id = store.insert(audio.clone()).await;
        let retrieved = store.get(id).await;

        assert_eq!(retrieved, Some(audio));
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown_id() {
        let store = AudioStore::new();
        let unknown = Uuid::new_v4();
        assert_eq!(store.get(unknown).await, None);
    }

    #[tokio::test]
    async fn sweep_does_not_remove_fresh_entries() {
        let store = AudioStore::new();
        let id = store.insert(b"audio".to_vec()).await;

        store.sweep().await;

        assert!(
            store.get(id).await.is_some(),
            "fresh entry should survive sweep"
        );
    }

    #[tokio::test]
    async fn multiple_inserts_return_distinct_ids() {
        let store = AudioStore::new();
        let id1 = store.insert(b"a".to_vec()).await;
        let id2 = store.insert(b"b".to_vec()).await;
        assert_ne!(id1, id2);
        assert_eq!(store.get(id1).await, Some(b"a".to_vec()));
        assert_eq!(store.get(id2).await, Some(b"b".to_vec()));
    }
}
