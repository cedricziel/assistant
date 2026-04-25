# Design: Gemini Provider

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       GEMINI PROVIDER DATA FLOW                             │
│                                                                             │
│  config.toml                                                                │
│  ┌─────────────────────┐                                                   │
│  │ [llm]               │                                                   │
│  │ provider = "gemini"  │                                                   │
│  │ model = "gemini-2.5- │                                                   │
│  │         flash"       │                                                   │
│  │ api_key = "AI..."    │                                                   │
│  └─────────┬───────────┘                                                   │
│            │                                                                │
│            ▼                                                                │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │  GeminiProvider                                                  │       │
│  │                                                                  │       │
│  │  ┌──────────────────────────────────────────────────────────┐   │       │
│  │  │  Request Builder                                          │   │       │
│  │  │                                                           │   │       │
│  │  │  ChatHistoryMessage[]  ──►  contents: [{                  │   │       │
│  │  │                                role: "user",              │   │       │
│  │  │                                parts: [{text: "..."}]     │   │       │
│  │  │                              }]                           │   │       │
│  │  │                                                           │   │       │
│  │  │  ToolSpec[]  ──►  tools: [{                               │   │       │
│  │  │                      functionDeclarations: [{             │   │       │
│  │  │                        name, description, parameters     │   │       │
│  │  │                      }]                                   │   │       │
│  │  │                   }]                                      │   │       │
│  │  │                                                           │   │       │
│  │  │  ToolCallItem (result)  ──►  parts: [{                   │   │       │
│  │  │                                functionResponse: {        │   │       │
│  │  │                                  name, id, response       │   │       │
│  │  │                                }                          │   │       │
│  │  │                              }]                           │   │       │
│  │  └──────────────────────────────────────────────────────────┘   │       │
│  │                              │                                   │       │
│  │                              ▼                                   │       │
│  │  ┌────────────────────────────────────────────────────────┐     │       │
│  │  │  HTTP (reqwest)                                         │     │       │
│  │  │                                                         │     │       │
│  │  │  POST /v1beta/models/{model}:generateContent            │     │       │
│  │  │  POST /v1beta/models/{model}:streamGenerateContent      │     │       │
│  │  │  POST /v1beta/models/{model}:embedContent               │     │       │
│  │  │                                                         │     │       │
│  │  │  Header: x-goog-api-key: {api_key}                     │     │       │
│  │  │  Header: Content-Type: application/json                 │     │       │
│  │  └────────────────────────────────────────────────────────┘     │       │
│  │                              │                                   │       │
│  │                              ▼                                   │       │
│  │  ┌────────────────────────────────────────────────────────┐     │       │
│  │  │  Response Parser                                        │     │       │
│  │  │                                                         │     │       │
│  │  │  candidates[0].content.parts[]                          │     │       │
│  │  │    ├── {text: "..."}  ──►  LlmResponse::FinalAnswer     │     │       │
│  │  │    └── {functionCall:                                   │     │       │
│  │  │          {id, name, args}}  ──►  LlmResponse::ToolCalls │     │       │
│  │  │                                                         │     │       │
│  │  │  usageMetadata  ──►  LlmResponseMeta                   │     │       │
│  │  └────────────────────────────────────────────────────────┘     │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Wire Format Mapping

### Messages: ChatHistoryMessage → Gemini contents

```
    OUR TYPE                          GEMINI WIRE FORMAT
    ════════                          ══════════════════

    ChatHistoryMessage::Text          {
      { role: User,                     "role": "user",
        content: "hello" }              "parts": [{"text": "hello"}]
                                      }

    ChatHistoryMessage::Text          {
      { role: Assistant,                "role": "model",
        content: "hi" }                 "parts": [{"text": "hi"}]
                                      }

    ChatHistoryMessage::               {
      MultimodalUser {                   "role": "user",
        content: [                       "parts": [
          Text("describe"),                {"text": "describe"},
          Image { media_type,              {"inlineData": {
                  data }                      "mimeType": "image/png",
        ]                                     "data": "<base64>"
      }                                     }}
                                         ]
                                      }

    ChatHistoryMessage::               {
      AssistantToolCalls([               "role": "model",
        ToolCallItem {                   "parts": [{
          name: "file-read",               "functionCall": {
          params: {path: "/tmp"},            "id": "call_123",
          id: Some("call_123")               "name": "file-read",
        }                                    "args": {"path": "/tmp"}
      ])                                   }
                                         }]
                                      }

    ChatHistoryMessage::               {
      ToolResult {                       "role": "user",
        name: "file-read",              "parts": [{
        content: "hello world"            "functionResponse": {
      }                                     "name": "file-read",
                                            "id": "call_123",
                                            "response": {"result": "hello world"}
                                          }
                                        }]
                                      }

    System prompt                     system_instruction: {
                                        parts: [{"text": "..."}]
                                      }
```

Key differences from OpenAI:

- **System prompt** goes in `system_instruction`, not as a message
- **Assistant** role is called `"model"`
- **Tool results** go in `"user"` role messages with `functionResponse` parts
- **Function args** are a JSON object, not a JSON string (simpler)
- **Function IDs** are mandatory in Gemini 3 — must track and pass back

### Tools: ToolSpec → Gemini functionDeclarations

```rust
fn build_gemini_tools(tools: &[ToolSpec]) -> Vec<Value> {
    if tools.is_empty() {
        return vec![];
    }

    let declarations: Vec<Value> = tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.normalized_params_schema(),
            })
        })
        .collect();

    vec![json!({ "functionDeclarations": declarations })]
}
```

### Google Search grounding tool

```rust
fn build_gemini_tools_with_search(
    tools: &[ToolSpec],
    google_search: bool,
) -> Vec<Value> {
    let mut gemini_tools = build_gemini_tools(tools);

    if google_search {
        gemini_tools.push(json!({
            "googleSearch": {}
        }));
    }

    gemini_tools
}
```

## Key Type Changes

### GeminiProvider

```rust
pub struct GeminiProvider {
    http_client: reqwest_middleware::ClientWithMiddleware,
    config: GeminiConfig,
}

pub struct GeminiConfig {
    pub model: String,
    pub api_key: String,
    pub base_url: String,    // default: https://generativelanguage.googleapis.com
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub google_search: bool,
    pub embedding_model: String,
}
```

### Config types

```rust
/// Provider-specific Gemini options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiOptions {
    /// Enable Google Search grounding (default: false).
    #[serde(default)]
    pub google_search: bool,
    /// Max output tokens (default: 8192).
    pub max_tokens: Option<u32>,
}
```

### LlmProviderKind + EmbeddingProviderKind

```rust
pub enum LlmProviderKind {
    #[default]
    Ollama,
    Anthropic,
    OpenAI,
    Moonshot,
    Gemini,      // NEW
    OpenRouter,  // from openrouter-provider change
}

pub enum EmbeddingProviderKind {
    Ollama,
    OpenAI,
    Voyage,
    Gemini,      // NEW
}
```

## Provider Implementation

### chat() — non-streaming

```rust
async fn chat(
    &self,
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    tools: &[ToolSpec],
) -> anyhow::Result<LlmResponse> {
    let url = format!(
        "{}/v1beta/models/{}:generateContent",
        self.config.base_url, self.config.model
    );

    let contents = build_gemini_contents(history);
    let gemini_tools = build_gemini_tools_with_search(tools, self.config.google_search);

    let mut body = json!({
        "contents": contents,
        "generationConfig": {
            "maxOutputTokens": self.config.max_tokens,
        },
    });

    if !system_prompt.is_empty() {
        body["systemInstruction"] = json!({
            "parts": [{"text": system_prompt}]
        });
    }
    if !gemini_tools.is_empty() {
        body["tools"] = json!(gemini_tools);
    }

    let resp = self.http_client
        .post(&url)
        .header("x-goog-api-key", &self.config.api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let resp_body: Value = resp.json().await?;

    if !status.is_success() {
        let msg = resp_body["error"]["message"].as_str().unwrap_or("unknown");
        anyhow::bail!("Gemini API error ({}): {}", status, msg);
    }

    parse_gemini_response(&resp_body)
}
```

### chat_streaming() — SSE

```rust
async fn chat_streaming(
    &self,
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    tools: &[ToolSpec],
    chunk_sink: Option<mpsc::Sender<StreamChunk>>,
) -> anyhow::Result<LlmResponse> {
    let url = format!(
        "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
        self.config.base_url, self.config.model
    );

    // ... same request body construction as chat() ...

    let resp = self.http_client
        .post(&url)
        .header("x-goog-api-key", &self.config.api_key)
        .json(&body)
        .send()
        .await?;

    // Gemini SSE: each event is a complete generateContent response
    // with incremental candidates[0].content.parts
    let mut text_buf = String::new();
    let mut tool_calls: Vec<ToolCallItem> = Vec::new();
    let mut meta = LlmResponseMeta::default();

    // Parse SSE stream line by line
    let mut stream = resp.bytes_stream();
    let mut line_buf = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        line_buf.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete SSE events (data: {...}\n\n)
        while let Some(pos) = line_buf.find("\n\n") {
            let event = line_buf[..pos].to_string();
            line_buf = line_buf[pos + 2..].to_string();

            if let Some(data) = event.strip_prefix("data: ") {
                let chunk_json: Value = serde_json::from_str(data)?;
                // Extract text deltas and function calls from parts
                // Send StreamChunk::Text for text parts
                // Accumulate functionCall parts
                // Extract usageMetadata for meta
            }
        }
    }

    // Return accumulated result
    if !tool_calls.is_empty() {
        Ok(LlmResponse::ToolCalls(ToolCallResponse {
            items: tool_calls,
            meta,
            thinking: None,
        }))
    } else {
        Ok(LlmResponse::FinalAnswer(text_buf, meta))
    }
}
```

### embed()

```rust
async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
    let model = &self.config.embedding_model;
    let url = format!(
        "{}/v1beta/models/{}:embedContent",
        self.config.base_url, model
    );

    let body = json!({
        "content": {
            "parts": [{"text": text}]
        }
    });

    let resp = self.http_client
        .post(&url)
        .header("x-goog-api-key", &self.config.api_key)
        .json(&body)
        .send()
        .await?;

    let resp_body: Value = resp.json().await?;
    let values = resp_body["embedding"]["values"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Gemini: missing embedding values"))?;

    values
        .iter()
        .map(|v| v.as_f64().map(|f| f as f32)
            .ok_or_else(|| anyhow::anyhow!("Gemini: invalid embedding value")))
        .collect()
}
```

## Response Parsing

```rust
fn parse_gemini_response(body: &Value) -> anyhow::Result<LlmResponse> {
    let candidate = body["candidates"]
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("Gemini: empty candidates"))?;

    let parts = candidate["content"]["parts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Gemini: missing parts"))?;

    let meta = LlmResponseMeta {
        model: body["modelVersion"].as_str().map(String::from),
        response_id: None,  // Gemini doesn't return a response ID
        finish_reason: candidate["finishReason"].as_str().map(|s| s.to_lowercase()),
        input_tokens: body["usageMetadata"]["promptTokenCount"].as_u64(),
        output_tokens: body["usageMetadata"]["candidatesTokenCount"].as_u64(),
    };

    // Check for function calls first
    let tool_calls: Vec<ToolCallItem> = parts
        .iter()
        .filter_map(|part| {
            let fc = part.get("functionCall")?;
            Some(ToolCallItem {
                name: fc["name"].as_str()?.to_string(),
                params: fc["args"].clone(),  // already a Value, no parsing needed
                id: fc["id"].as_str().map(String::from),
            })
        })
        .collect();

    if !tool_calls.is_empty() {
        return Ok(LlmResponse::ToolCalls(ToolCallResponse {
            items: tool_calls,
            meta,
            thinking: None,
        }));
    }

    // Collect text parts
    let text: String = parts
        .iter()
        .filter_map(|part| part["text"].as_str())
        .collect::<Vec<_>>()
        .join("");

    Ok(LlmResponse::FinalAnswer(text, meta))
}
```

## Function ID Tracking

Gemini 3 requires function IDs in `functionResponse`. The existing `ToolCallItem.id: Option<String>` field carries this through the orchestrator. In `build_gemini_contents()`, when converting `ToolResult`, we look up the matching ID from the preceding `AssistantToolCalls`:

```rust
ChatHistoryMessage::ToolResult { name, content } => {
    // Find matching ID from pending_ids (same pattern as Moonshot)
    let pos = pending_ids.iter().position(|(n, _)| n == name);
    let call_id = if let Some(idx) = pos {
        pending_ids.remove(idx).1
    } else {
        warn!(tool = %name, "no pending function call ID for tool result");
        String::new()
    };

    let mut response_obj = json!({
        "name": name,
        "response": {"result": content},
    });
    if !call_id.is_empty() {
        response_obj["id"] = json!(call_id);
    }

    contents.push(json!({
        "role": "user",
        "parts": [{ "functionResponse": response_obj }]
    }));
}
```

## Gemini Embedding Provider

For standalone embedding use (separate from the LLM provider):

```rust
pub struct GeminiEmbedder {
    http_client: reqwest_middleware::ClientWithMiddleware,
    api_key: String,
    base_url: String,
    model: String,
}

impl GeminiEmbedder {
    pub fn from_embedding_config(cfg: &EmbeddingConfig) -> anyhow::Result<Self>;
}
```

Wired into the embedding provider factory alongside Voyage and OpenAI embedders.

## Testing Strategy

- **Unit tests**: Message builder tests for each `ChatHistoryMessage` variant → Gemini `contents` format. Tool spec conversion tests. Response parsing tests (text, tool calls, mixed).
- **Wiremock tests**: Full chat round-trip (request → mock response → `LlmResponse`). Streaming SSE accumulation. Error handling (4xx, 5xx). Embedding endpoint.
- **Config tests**: `LlmProviderKind::Gemini` deserializes from TOML. `create_provider()` creates `GeminiProvider`. `EmbeddingProviderKind::Gemini` works independently.
- **Integration test** (ignored, requires key): Live call to Gemini API with function calling.

## Performance Considerations

- Gemini's SSE format sends complete response objects per chunk (not deltas). Each chunk is a full `generateContent` response with incremental parts. This means slightly more parsing overhead per chunk compared to OpenAI's delta format, but the difference is negligible.
- The `embedContent` endpoint supports batch embedding (`batchEmbedContents`). Initial implementation does single-text embedding; batch can be added if needed for performance.

## Migration / Backwards Compatibility

- `LlmProviderKind::Gemini` and `EmbeddingProviderKind::Gemini` are new enum variants. No impact on existing configs.
- No new crate dependencies — uses existing `reqwest`, `serde_json`, `tokio`, `futures`.
- No database migrations.
