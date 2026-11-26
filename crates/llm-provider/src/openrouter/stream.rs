//! OpenRouter SSE stream parsing.

use serde::Deserialize;
use serde_json::Value;

use assistant_core::{LlmResponse, LlmResponseMeta, StreamChunk, ToolCallItem, ToolCallResponse};

use super::citations::sources_block;
use super::errors::{ErrorBody, body_error};
use super::media::{ImageEntry, collect_urls};
use super::reasoning::{budget_exhausted, merge_detail, provider_state, visible_text};
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
    error: Option<ErrorBody>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    #[serde(default)]
    delta: Delta,
    #[serde(default)]
    finish_reason: Option<String>,
    /// Some upstreams repeat the full message in the last chunk.
    #[serde(default)]
    message: Option<ChunkMessage>,
}

#[derive(Debug, Default, Deserialize)]
struct ChunkMessage {
    #[serde(default)]
    images: Vec<ImageEntry>,
}

#[derive(Debug, Default, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallDelta>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    reasoning_details: Vec<Value>,
    #[serde(default)]
    annotations: Vec<Value>,
    #[serde(default)]
    images: Vec<ImageEntry>,
    #[serde(default)]
    audio: Option<AudioDelta>,
}

/// A piece of a spoken reply: base64 PCM16 and the words it says.
#[derive(Debug, Default, Deserialize)]
struct AudioDelta {
    #[serde(default)]
    data: Option<String>,
    #[serde(default)]
    transcript: Option<String>,
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
    show_reasoning: bool,
    reasoned: bool,
    model: String,
    reasoning_details: Vec<Value>,
    annotations: Vec<Value>,
    image_urls: Vec<String>,
    /// PCM16 of a spoken reply, in arrival order.
    audio: Vec<u8>,
    transcript: String,
}

impl StreamAccumulator {
    /// With `show_reasoning` off, reasoning produces no chunks. `model` is
    /// the requested model, which stored reasoning is keyed by.
    pub fn new(model: &str, show_reasoning: bool) -> Self {
        Self {
            show_reasoning,
            model: model.to_owned(),
            ..Self::default()
        }
    }

    /// Whether `[DONE]` has been seen; later lines are ignored.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Feeds one SSE line and returns the chunks it carried: reasoning as
    /// [`StreamChunk::Thinking`], then answer text. Comments, blank lines and
    /// non-`data` fields are ignored.
    pub fn feed_line(&mut self, line: &str) -> anyhow::Result<Vec<StreamChunk>> {
        let Some(payload) = line.strip_prefix("data:") else {
            return Ok(Vec::new());
        };
        let payload = payload.trim();
        if payload == "[DONE]" {
            self.done = true;
            return Ok(Vec::new());
        }
        let chunk: Chunk = serde_json::from_str(payload)
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenRouter stream chunk: {e}"))?;
        if let Some(error) = &chunk.error {
            return Err(body_error(error));
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
        let mut thinking = String::new();
        let mut transcript_chunk = String::new();
        for choice in chunk.choices {
            let reasoning = visible_text(
                &choice.delta.reasoning_details,
                choice.delta.reasoning.as_deref(),
            );
            if !reasoning.is_empty() {
                self.reasoned = true;
                thinking.push_str(&reasoning);
            }
            for detail in choice.delta.reasoning_details {
                merge_detail(&mut self.reasoning_details, detail);
            }
            self.annotations.extend(choice.delta.annotations);
            collect_urls(&mut self.image_urls, choice.delta.images);
            if let Some(message) = choice.message {
                collect_urls(&mut self.image_urls, message.images);
            }
            if let Some(reason) = choice.finish_reason {
                if reason == "error" {
                    anyhow::bail!("OpenRouter stream finished with an error");
                }
                self.meta.finish_reason = Some(reason);
            }
            if let Some(content) = choice.delta.content {
                text.push_str(&content);
            }
            if let Some(audio) = choice.delta.audio {
                if let Some(data) = audio.data {
                    use base64::Engine as _;
                    let pcm = base64::engine::general_purpose::STANDARD
                        .decode(data.trim())
                        .map_err(|e| {
                            anyhow::anyhow!("OpenRouter returned invalid base64 audio: {e}")
                        })?;
                    self.audio.extend_from_slice(&pcm);
                }
                if let Some(words) = audio.transcript {
                    self.transcript.push_str(&words);
                    // A spoken reply's text arrives as its transcript.
                    if self.text.is_empty() && text.is_empty() {
                        transcript_chunk.push_str(&words);
                    }
                }
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
        let mut chunks = Vec::new();
        if self.show_reasoning && !thinking.is_empty() {
            chunks.push(StreamChunk::Thinking(thinking));
        }
        if !text.is_empty() {
            chunks.push(StreamChunk::Text(text));
        } else if !transcript_chunk.is_empty() {
            chunks.push(StreamChunk::Text(transcript_chunk));
        }
        Ok(chunks)
    }

    /// The Sources block a final answer ends with, to stream as its last
    /// chunk; [`finish`](Self::finish) appends the same block.
    pub fn sources_block(&self) -> Option<String> {
        if self.tool_calls.iter().any(|c| !c.name.is_empty()) {
            return None;
        }
        sources_block(&self.text, &self.annotations)
    }

    /// The PCM16 of the spoken reply streamed so far, if any.
    pub fn take_audio(&mut self) -> Option<Vec<u8>> {
        (!self.audio.is_empty()).then(|| std::mem::take(&mut self.audio))
    }

    /// The URLs of the images streamed so far, without duplicates.
    pub fn take_image_urls(&mut self) -> Vec<String> {
        std::mem::take(&mut self.image_urls)
    }

    /// The response, once the stream has ended. A stream that ends without
    /// `[DONE]` or a finish reason was cut off.
    pub fn finish(mut self) -> anyhow::Result<LlmResponse> {
        if self.text.is_empty() {
            self.text = std::mem::take(&mut self.transcript);
        }
        let sources = self.sources_block().unwrap_or_default();
        if !self.done && self.meta.finish_reason.is_none() {
            anyhow::bail!("OpenRouter stream ended before the response was complete");
        }
        let has_tool_calls = self.tool_calls.iter().any(|c| !c.name.is_empty());
        let reasoned = self.reasoned || self.meta.reasoning_tokens.unwrap_or(0) > 0;
        if let Some(error) = budget_exhausted(
            self.meta.finish_reason.as_deref(),
            &self.text,
            has_tool_calls,
            reasoned,
        ) {
            return Err(error);
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
                provider_state: provider_state(&self.model, self.reasoning_details),
            }));
        }
        Ok(LlmResponse::FinalAnswer(self.text + &sources, self.meta))
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::StreamChunk;

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
        for line in ["event: message", "id: 7", ": OPENROUTER PROCESSING"] {
            let chunks: Vec<StreamChunk> = acc.feed_line(line).unwrap();
            assert!(chunks.is_empty(), "{line}");
        }
    }
}
