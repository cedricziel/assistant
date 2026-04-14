//! Background indexer for conversation turn history.
//!
//! After each turn completes the orchestrator calls
//! [`index_conversation`], which runs as a fire-and-forget `tokio::spawn`
//! task off the critical path.
//!
//! # Storage scheme
//!
//! Turns are stored in the shared `memory_chunks` table using a virtual
//! file-path scheme:
//!
//! ```text
//! session://<conversation_id>/turn_<turn_number>
//! ```
//!
//! Each turn is treated as a one-chunk "file".  The SHA-256 of the
//! formatted turn text is used as the file hash, so an already-indexed
//! turn is never re-processed.  Because the paths share the
//! `memory_chunks` table, the existing `memory-search` tool surfaces
//! conversation history alongside memory files with no extra plumbing.
//!
//! # Filtering
//!
//! Only `user` and `assistant` roles are indexed.  Tool calls,
//! tool results, and `<think>…</think>` thinking steps are skipped —
//! they are noisy and rarely useful to recall.

use std::sync::Arc;

use anyhow::Result;
use assistant_core::MessageRole;
use assistant_llm::LlmProvider;
use assistant_storage::{MemoryChunkStore, StorageLayer};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};
use uuid::Uuid;

/// Spawn a fire-and-forget task that indexes the conversation.
///
/// Called by the orchestrator at the end of each successful turn.
pub(crate) fn spawn_index(
    conversation_id: Uuid,
    agent_id: String,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
) {
    tokio::spawn(async move {
        index_conversation(conversation_id, agent_id, storage, llm).await;
    });
}

/// Index all unindexed turns in a conversation into `memory_chunks`.
///
/// Designed to be called via `tokio::spawn` — any errors are logged and
/// swallowed so callers never block on indexing.
async fn index_conversation(
    conversation_id: Uuid,
    agent_id: String,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
) {
    if let Err(e) =
        index_conversation_inner(conversation_id, &agent_id, &storage, llm.as_ref()).await
    {
        warn!(
            %conversation_id,
            agent_id,
            error = %e,
            "Conversation indexing failed"
        );
    }
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

async fn index_conversation_inner(
    conversation_id: Uuid,
    agent_id: &str,
    storage: &StorageLayer,
    llm: &dyn LlmProvider,
) -> Result<()> {
    let conv_store = storage.conversation_store_for_agent(agent_id);
    let chunk_store = storage.memory_chunks_store();

    let messages = conv_store.load_history(conversation_id).await?;

    for msg in &messages {
        // Only index human-readable roles.
        if !matches!(msg.role, MessageRole::User | MessageRole::Assistant) {
            continue;
        }

        let content = msg.content.trim();

        // Skip empty content, tool call JSON blobs, and thinking steps.
        if content.is_empty()
            || content.starts_with('{')
            || content.starts_with('<')
            || content.starts_with('[')
        {
            continue;
        }

        let turn_num = msg.turn;
        let file_path = format!(
            "session://{}/{}/turn_{}",
            &conversation_id.to_string()[..8],
            conversation_id,
            turn_num
        );

        let role_label = match msg.role {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            _ => continue,
        };

        let formatted = format!("[{role_label} • turn {turn_num}]\n{content}");
        let hash = sha256_hex(&formatted);

        // Skip if already indexed with the same content.
        if chunk_store.get_file_hash(&file_path).await? == Some(hash.clone()) {
            debug!(file_path, "Turn already indexed, skipping");
            continue;
        }

        // Store as a single chunk (turns are short enough not to split).
        let chunks: Vec<String> = split_turn(&formatted);
        chunk_store
            .replace_file_chunks(&file_path, &hash, &chunks)
            .await?;

        debug!(
            file_path,
            chunks = chunks.len(),
            "Indexed conversation turn"
        );
    }

    // Embed any unembedded chunks from this conversation (best-effort).
    embed_pending(&chunk_store, llm).await;

    Ok(())
}

/// Split a turn into chunks ≤ 400 bytes, respecting char boundaries.
fn split_turn(text: &str) -> Vec<String> {
    const MAX: usize = 400;
    if text.len() <= MAX {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = (start + MAX).min(text.len());
        let mut end = end;
        while end > start && !text.is_char_boundary(end) {
            end -= 1;
        }
        chunks.push(text[start..end].to_string());
        start = end;
    }
    chunks
}

/// Embed unembedded chunks (same logic as `MemoryIndexer::embed_pending`).
async fn embed_pending(store: &MemoryChunkStore, llm: &dyn LlmProvider) {
    let unembedded = match store.get_unembedded(50).await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "Failed to load unembedded chunks");
            return;
        }
    };

    for chunk in unembedded {
        match llm.embed(&chunk.content).await {
            Ok(vec) => {
                if let Err(e) = store.update_embedding(chunk.id, &vec).await {
                    warn!(chunk_id = chunk.id, error = %e, "Failed to store embedding");
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not supported") || msg.contains("Not supported") {
                    debug!("Embedding unsupported, skipping batch");
                    break;
                }
                debug!(chunk_id = chunk.id, error = %e, "Embedding failed, skipping");
            }
        }
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_turn_short_text_is_single_chunk() {
        let text = "Hello world";
        let chunks = split_turn(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn split_turn_long_text_is_split() {
        let text = "x".repeat(900);
        let chunks = split_turn(&text);
        assert!(chunks.len() > 1, "expected multiple chunks");
        for c in &chunks {
            assert!(c.len() <= 400, "chunk too long: {}", c.len());
        }
        assert_eq!(chunks.concat(), text, "chunks must reassemble to original");
    }

    #[test]
    fn split_turn_multibyte_boundary() {
        let text = "🦀".repeat(120); // 120 × 4 bytes = 480, needs one split
        let chunks = split_turn(&text);
        for c in &chunks {
            assert!(c.len() <= 400, "chunk too long: {}", c.len());
            // Must be valid UTF-8 (no panic on chars()).
            let _ = c.chars().count();
        }
    }

    #[test]
    fn sha256_is_stable() {
        assert_eq!(sha256_hex("hello"), sha256_hex("hello"));
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }

    // -- Integration: full pipeline with in-memory DB --

    struct NoEmbedLlm;

    #[async_trait::async_trait]
    impl assistant_llm::LlmProvider for NoEmbedLlm {
        fn provider_name(&self) -> &str {
            "test"
        }
        fn capabilities(&self) -> assistant_llm::Capabilities {
            assistant_llm::Capabilities {
                tools: assistant_llm::ToolSupport::None,
                streaming: false,
                vision: false,
                hosted_tools: vec![],
            }
        }
        async fn chat(
            &self,
            _system: &str,
            _history: &[assistant_llm::ChatHistoryMessage],
            _tools: &[assistant_llm::ToolSpec],
        ) -> anyhow::Result<assistant_llm::LlmResponse> {
            anyhow::bail!("not implemented")
        }
        async fn chat_streaming(
            &self,
            _system: &str,
            _history: &[assistant_llm::ChatHistoryMessage],
            _tools: &[assistant_llm::ToolSpec],
            _sink: Option<tokio::sync::mpsc::Sender<String>>,
        ) -> anyhow::Result<assistant_llm::LlmResponse> {
            anyhow::bail!("not implemented")
        }
    }

    #[tokio::test]
    async fn indexes_user_and_assistant_messages() {
        use assistant_core::Message;
        use uuid::Uuid;

        let storage = Arc::new(
            assistant_storage::StorageLayer::new_in_memory()
                .await
                .expect("in-memory DB"),
        );
        let _llm: Arc<dyn assistant_llm::LlmProvider> = Arc::new(NoEmbedLlm);

        let conv_id = Uuid::new_v4();
        let agent_id = "test-agent".to_string();
        let conv_store = storage.conversation_store_for_agent(&agent_id);

        // Create the conversation and persist two messages.
        conv_store
            .create_conversation_with_id(conv_id, Some("test"))
            .await
            .unwrap();

        let mut user_msg = Message::user(conv_id, "What is the capital of France?");
        user_msg.turn = 1;
        conv_store.save_message(&user_msg).await.unwrap();

        let mut asst_msg = Message::assistant(conv_id, "The capital of France is Paris.");
        asst_msg.turn = 2;
        conv_store.save_message(&asst_msg).await.unwrap();

        // Run the indexer.
        index_conversation_inner(conv_id, &agent_id, &storage, &NoEmbedLlm)
            .await
            .expect("indexing should succeed");

        // Both turns should now be in memory_chunks.
        let chunk_store = storage.memory_chunks_store();
        let count = chunk_store.count().await.unwrap();
        assert!(count >= 2, "expected at least 2 chunks, got {count}");

        // Running again should be idempotent (hash unchanged → skip).
        index_conversation_inner(conv_id, &agent_id, &storage, &NoEmbedLlm)
            .await
            .expect("re-indexing should succeed");
        let count2 = chunk_store.count().await.unwrap();
        assert_eq!(count, count2, "idempotent: count should not change");
    }

    #[tokio::test]
    async fn skips_tool_json_and_thinking_steps() {
        use assistant_core::Message;
        use uuid::Uuid;

        let storage = Arc::new(
            assistant_storage::StorageLayer::new_in_memory()
                .await
                .expect("in-memory DB"),
        );
        let conv_id = Uuid::new_v4();
        let agent_id = "test-agent".to_string();
        let conv_store = storage.conversation_store_for_agent(&agent_id);

        conv_store
            .create_conversation_with_id(conv_id, Some("test"))
            .await
            .unwrap();

        // Should be indexed.
        let mut user_msg = Message::user(conv_id, "Hello");
        user_msg.turn = 1;
        conv_store.save_message(&user_msg).await.unwrap();

        // Should be skipped (starts with '<').
        let mut thinking = Message::assistant(conv_id, "<think>internal reasoning</think>");
        thinking.turn = 2;
        conv_store.save_message(&thinking).await.unwrap();

        // Should be skipped (starts with '{').
        let mut tool_json = Message::assistant(conv_id, r#"{"tool":"file-read"}"#);
        tool_json.turn = 3;
        conv_store.save_message(&tool_json).await.unwrap();

        index_conversation_inner(conv_id, &agent_id, &storage, &NoEmbedLlm)
            .await
            .unwrap();

        // Only the user message should be indexed.
        let chunk_store = storage.memory_chunks_store();
        let count = chunk_store.count().await.unwrap();
        assert_eq!(count, 1, "expected only user message indexed, got {count}");
    }
}
