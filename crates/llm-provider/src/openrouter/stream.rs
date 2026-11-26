//! OpenRouter SSE stream parsing.

use serde::Deserialize;
use serde_json::Value;

use assistant_core::{LlmResponse, LlmResponseMeta, ToolCallItem, ToolCallResponse};

use super::wire::Usage;

/// Splits a byte stream into lines, decoding each line only once it is
/// complete, so multi-byte characters split across chunks stay intact.
#[derive(Default)]
pub struct LineBuffer {
    buf: Vec<u8>,
}

impl LineBuffer {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(bytes);
        let mut lines = Vec::new();
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&line[..line.len() - 1]);
            lines.push(text.trim_end_matches('\r').to_owned());
        }
        lines
    }
}

#[derive(Debug, Deserialize)]
struct Chunk {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    choices: Vec<ChunkChoice>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    error: Option<StreamError>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    #[serde(default)]
    delta: Delta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallDelta>,
}

#[derive(Debug, Deserialize)]
struct ToolCallDelta {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<FunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct FunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamError {
    #[serde(default)]
    code: Value,
    #[serde(default)]
    message: String,
}

#[derive(Default)]
struct PartialToolCall {
    id: Option<String>,
    name: String,
    arguments: String,
}

/// Folds SSE lines into text deltas and the final response.
#[derive(Default)]
pub struct StreamAccumulator {
    text: String,
    tool_calls: Vec<PartialToolCall>,
    meta: LlmResponseMeta,
    done: bool,
}

impl StreamAccumulator {
    /// Whether `[DONE]` has been seen; later lines are ignored.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Feeds one SSE line and returns the text delta it carried, if any.
    /// Comments, blank lines and non-`data` fields are ignored.
    pub fn feed_line(&mut self, line: &str) -> anyhow::Result<Option<String>> {
        let Some(payload) = line.strip_prefix("data:") else {
            return Ok(None);
        };
        let payload = payload.trim();
        if payload == "[DONE]" {
            self.done = true;
            return Ok(None);
        }
        let chunk: Chunk = serde_json::from_str(payload)
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenRouter stream chunk: {e}"))?;
        if let Some(error) = chunk.error {
            anyhow::bail!(
                "OpenRouter stream error ({}): {}",
                error.code,
                error.message
            );
        }
        if chunk.id.is_some() {
            self.meta.response_id = chunk.id;
        }
        if chunk.model.is_some() {
            self.meta.model = chunk.model;
        }
        if let Some(usage) = &chunk.usage {
            usage.apply(&mut self.meta);
        }

        let mut text = String::new();
        for choice in chunk.choices {
            if let Some(reason) = choice.finish_reason {
                if reason == "error" {
                    anyhow::bail!("OpenRouter stream finished with an error");
                }
                self.meta.finish_reason = Some(reason);
            }
            if let Some(content) = choice.delta.content {
                text.push_str(&content);
            }
            for delta in choice.delta.tool_calls {
                if self.tool_calls.len() <= delta.index {
                    self.tool_calls
                        .resize_with(delta.index + 1, PartialToolCall::default);
                }
                let call = &mut self.tool_calls[delta.index];
                if delta.id.is_some() {
                    call.id = delta.id;
                }
                if let Some(function) = delta.function {
                    if let Some(name) = function.name {
                        call.name.push_str(&name);
                    }
                    if let Some(arguments) = function.arguments {
                        call.arguments.push_str(&arguments);
                    }
                }
            }
        }
        self.text.push_str(&text);
        Ok((!text.is_empty()).then_some(text))
    }

    /// The response, once the stream has ended. A stream that ends without
    /// `[DONE]` or a finish reason was cut off.
    pub fn finish(self) -> anyhow::Result<LlmResponse> {
        if !self.done && self.meta.finish_reason.is_none() {
            anyhow::bail!("OpenRouter stream ended before the response was complete");
        }
        let items: Vec<ToolCallItem> = self
            .tool_calls
            .into_iter()
            .filter(|c| !c.name.is_empty())
            .map(|c| ToolCallItem {
                params: serde_json::from_str(&c.arguments)
                    .unwrap_or_else(|_| Value::Object(serde_json::Map::new())),
                name: c.name,
                id: c.id,
            })
            .collect();
        if !items.is_empty() {
            return Ok(LlmResponse::ToolCalls(ToolCallResponse {
                items,
                meta: self.meta,
                thinking: None,
                provider_state: None,
            }));
        }
        Ok(LlmResponse::FinalAnswer(self.text, self.meta))
    }
}

#[cfg(test)]
mod tests {
    use super::{LineBuffer, StreamAccumulator};

    #[test]
    fn multibyte_character_split_across_chunks_is_kept() {
        let bytes = "data: é\n".as_bytes();
        let split = bytes.iter().position(|&b| b == 0xC3).unwrap() + 1;
        let mut buffer = LineBuffer::default();

        assert!(
            buffer.push(&bytes[..split]).is_empty(),
            "no complete line yet"
        );
        assert_eq!(buffer.push(&bytes[split..]), ["data: é"]);
    }

    #[test]
    fn crlf_line_endings_are_trimmed() {
        let mut buffer = LineBuffer::default();
        assert_eq!(buffer.push(b"data: x\r\n\r\n"), ["data: x", ""]);
    }

    #[test]
    fn lines_after_done_are_flagged() {
        let mut acc = StreamAccumulator::default();
        acc.feed_line("data: [DONE]").unwrap();
        assert!(acc.is_done());
    }

    #[test]
    fn event_and_id_fields_are_ignored() {
        let mut acc = StreamAccumulator::default();
        assert_eq!(acc.feed_line("event: message").unwrap(), None);
        assert_eq!(acc.feed_line("id: 7").unwrap(), None);
        assert_eq!(acc.feed_line(": OPENROUTER PROCESSING").unwrap(), None);
    }
}
