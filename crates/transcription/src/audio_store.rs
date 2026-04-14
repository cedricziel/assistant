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
