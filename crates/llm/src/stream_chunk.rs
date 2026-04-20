//! Typed chunks emitted during LLM streaming.
//!
//! [`StreamChunk`] replaces the raw `String` token sink, allowing providers
//! to distinguish between text output and internal reasoning tokens.

/// A typed chunk emitted during LLM streaming.
///
/// Providers send these through an `mpsc::Sender<StreamChunk>` so that
/// downstream consumers (orchestrator, web UI) can differentiate between
/// final-answer text and internal reasoning without guessing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamChunk {
    /// Incremental text token (part of the final answer visible to the user).
    Text(String),
    /// Incremental thinking/reasoning token (internal chain-of-thought).
    Thinking(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_chunk_stores_content() {
        let chunk = StreamChunk::Text("hello".to_string());
        assert_eq!(chunk, StreamChunk::Text("hello".to_string()));
    }

    #[test]
    fn thinking_chunk_stores_content() {
        let chunk = StreamChunk::Thinking("let me consider".to_string());
        assert_eq!(chunk, StreamChunk::Thinking("let me consider".to_string()));
    }

    #[test]
    fn chunks_are_cloneable_and_debuggable() {
        let chunks = vec![
            StreamChunk::Text("hi".to_string()),
            StreamChunk::Thinking("hmm".to_string()),
        ];
        for chunk in &chunks {
            let cloned = chunk.clone();
            let debug = format!("{:?}", cloned);
            assert!(!debug.is_empty());
        }
    }
}
