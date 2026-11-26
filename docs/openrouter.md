# OpenRouter Provider

Connects the assistant to [OpenRouter](https://openrouter.ai/), a unified
API gateway providing access to 300+ models from Anthropic, OpenAI, Meta,
Google, Mistral, and many more — all through a single API key and the
OpenAI Chat Completions wire format.

## Quick start

```sh
# Set your API key (or put it in org.toml)
export OPENROUTER_API_KEY=sk-or-...

# Edit ~/.assistant/orgs/default/org.toml
mkdir -p ~/.assistant/orgs/default
cat <<'EOF' >> ~/.assistant/orgs/default/org.toml
[llm]
provider = "openrouter"
model    = "anthropic/claude-sonnet-4-20250514"
EOF

# Run the assistant
cargo run -p assistant-cli
```

## Configuration

All OpenRouter settings live under the `[llm]` and `[llm.openrouter]`
sections in `~/.assistant/orgs/default/org.toml`.

### `[llm]` section

| Key              | Env var              | Default                          | Description                                   |
| ---------------- | -------------------- | -------------------------------- | --------------------------------------------- |
| `provider`       |                      | `"ollama"`                       | Set to `"openrouter"` to use this provider    |
| `model`          |                      | _(required)_                     | OpenRouter model slug (see [Models](#models)) |
| `api_key`        | `OPENROUTER_API_KEY` | _(required)_                     | API key; env var is checked as fallback       |
| `base_url`       |                      | `"https://openrouter.ai/api/v1"` | API base URL                                  |
| `max_iterations` |                      | `80`                             | Max tool-calling iterations per turn          |
| `timeout_secs`   |                      | `120`                            | Timeout for each LLM call (seconds)           |

### `[llm.openrouter]` section

| Key               | Default  | Description                                                                                        |
| ----------------- | -------- | -------------------------------------------------------------------------------------------------- |
| `referer`         | _(none)_ | `HTTP-Referer` header, used by OpenRouter for app attribution                                      |
| `title`           | _(none)_ | `X-OpenRouter-Title` header, shown in the OpenRouter dashboard                                     |
| `categories`      | `[]`     | `X-OpenRouter-Categories` header, comma-joined. At most two; more fails config load                |
| `max_tokens`      | `8192`   | Maximum completion tokens per response, sent as `max_completion_tokens`                            |
| `fallback_models` | `[]`     | Models to try in order when `model` fails. Sent as `models` with `model` first; omitted when empty |
| `pdf_engine`      | unset    | PDF parser: `"native"`, `"cloudflare-ai"` or `"mistral-ocr"`. See [PDFs](#pdfs)                    |
| `image_output`    | `false`  | Ask the model to answer with images. See [Chat image output](#chat-image-output)                   |

### `[llm.openrouter.provider]` section

Provider routing preferences, sent as the request's `provider` object.
Only the keys you set are sent. An unknown key fails config load.

| Key                  | Type                                              | Description                                           |
| -------------------- | ------------------------------------------------- | ----------------------------------------------------- |
| `order`              | list of provider slugs                            | Providers to try, in this order                       |
| `only`               | list of provider slugs                            | Use only these providers                              |
| `ignore`             | list of provider slugs                            | Never use these providers                             |
| `allow_fallbacks`    | boolean                                           | Whether OpenRouter may fall back to other providers   |
| `require_parameters` | boolean                                           | Use only providers that support every request field   |
| `data_collection`    | `"allow"` or `"deny"`                             | Whether providers that store or train on data qualify |
| `zdr`                | boolean                                           | Use only zero-data-retention endpoints                |
| `sort`               | `"price"`, `"throughput"` or `"latency"`          | How to rank providers                                 |
| `quantizations`      | list                                              | Accepted quantization levels                          |
| `max_price`          | table: `prompt`, `completion`, `request`, `image` | Highest acceptable prices; unknown keys fail load     |

### `[llm.openrouter.reasoning]` section

Reasoning settings, sent as the request's `reasoning` object. Only the
keys you set are sent. An unknown key fails config load.

| Key          | Type                                                                       | Description                            |
| ------------ | -------------------------------------------------------------------------- | -------------------------------------- |
| `effort`     | `"none"`, `"minimal"`, `"low"`, `"medium"`, `"high"`, `"xhigh"` or `"max"` | How hard the model reasons             |
| `max_tokens` | integer                                                                    | Token budget for reasoning             |
| `exclude`    | boolean                                                                    | Reason without returning the reasoning |
| `enabled`    | boolean                                                                    | Turn reasoning on or off               |

Setting both `effort` and `max_tokens` fails config load with
`` `effort` and `max_tokens` cannot both be set ``.

When `effort` is set, the provider checks it against the model's entry
in the catalog (see [Capabilities](#capabilities)). If the model can't
take that effort, the provider logs the reason followed by
`; chat calls will fail`, and every chat call fails with that reason
without contacting OpenRouter. See
[Troubleshooting](#troubleshooting) for the messages.

### `[llm.openrouter.image_config]` section

Sent as the request's `image_config` when `image_output = true`. Only the
keys you set are sent. An unknown key fails config load. See
[Chat image output](#chat-image-output).

| Key            | Type   | Description                                |
| -------------- | ------ | ------------------------------------------ |
| `aspect_ratio` | string | Image proportions, for example `"16:9"`    |
| `image_size`   | string | Image resolution: `"1K"`, `"2K"` or `"4K"` |

### `[llm.openrouter.audio_input]` section

Sends voice notes to the model as audio. An unknown key fails config
load. See [Voice](#voice).

| Key        | Default          | Description                                                         |
| ---------- | ---------------- | ------------------------------------------------------------------- |
| `enabled`  | `false`          | Send the recording of a voice turn                                  |
| `formats`  | `["wav", "mp3"]` | Formats sent unchanged. Any other format is converted to 16 kHz WAV |
| `max_secs` | `600`            | Longer recordings are sent as text only                             |

### `[llm.openrouter.audio_output]` section

Asks the model to speak its reply to a voice turn. An unknown key fails
config load. See [Voice](#voice).

| Key               | Default   | Description                                |
| ----------------- | --------- | ------------------------------------------ |
| `enabled`         | `false`   | Ask for a spoken reply on voice turns      |
| `voice`           | `"alloy"` | Voice sent in the request's `audio` object |
| `pcm_sample_rate` | `24000`   | Sample rate of the model's raw PCM, in Hz  |

### `[llm.openrouter.cache]` section

Prompt caching settings. An unknown key fails config load. See
[Prompt caching](#prompt-caching).

| Key       | Default | Description                                                               |
| --------- | ------- | ------------------------------------------------------------------------- |
| `enabled` | `true`  | Send `cache_control` breakpoints. `false` sends none                      |
| `ttl`     | `"5m"`  | Cache lifetime, `"5m"` or `"1h"`. `"1h"` applies to Anthropic models only |

### `[llm.openrouter.web_search]` section

OpenRouter's `web` plugin. Only set keys are sent. An unknown key fails
config load. See [Web search](#web-search).

| Key                   | Default | Description                                                                                      |
| --------------------- | ------- | ------------------------------------------------------------------------------------------------ |
| `enabled`             | `false` | Send the `web` plugin                                                                            |
| `engine`              | unset   | `"native"`, `"exa"`, `"firecrawl"`, `"parallel"` or `"perplexity"`. Unset lets OpenRouter choose |
| `max_results`         | unset   | Maximum search results                                                                           |
| `search_prompt`       | unset   | Prompt that introduces the results to the model                                                  |
| `include_domains`     | unset   | Domains to search                                                                                |
| `exclude_domains`     | unset   | Domains to skip                                                                                  |
| `search_context_size` | unset   | `"low"`, `"medium"` or `"high"`. Needs `engine = "native"` or no `engine`                        |

Which domain filters each engine honours is up to OpenRouter; see
[OpenRouter's web search docs](https://openrouter.ai/docs/features/web-search).

## Web search

With `enabled = true`, every request carries
`plugins: [{"id": "web", ...}]` with the options you set.
`search_context_size` goes in the top-level
`web_search_options: {"search_context_size": ...}` instead. The provider
reports web search as a hosted tool, so the model is not offered the
local `web-search` tool. With `enabled = false`, no plugin is sent and
the local tool stays available.

When a final answer carries `url_citation` annotations, it ends with a
`Sources:` list. Each cited URL appears once, in order of first
citation, as a markdown link labelled with its title, or the URL when
there is no title. Brackets in titles are escaped. URLs the answer
already links inline are not listed again. Responses that call tools
get no list. When streaming, the list is the last text chunk, so the
saved answer matches what was streamed.

## PDFs

A PDF attachment goes in the user message as a `file` part:

```json
{
  "type": "file",
  "file": {
    "filename": "document.pdf",
    "file_data": "data:application/pdf;base64,..."
  }
}
```

Attachments carry no filename, so it is always `document.pdf`. Other
document types are not sent.

A request with a PDF carries
`plugins: [{"id": "file-parser", "pdf": {"engine": ...}}]`:

- With `pdf_engine` set, that engine is used for every model. Use
  `"mistral-ocr"` for scanned PDFs.
- With `pdf_engine` unset, a model whose catalog entry lists `file` in
  its input modalities reads the PDF itself and gets no plugin. Other
  models get `"cloudflare-ai"`. Until the catalog loads, or when it
  can't be reached, every model counts as having no `file` input.

Requests without a PDF get no parser plugin. With web search on, both
plugins are sent, `web` first.

## Request identity

During a conversation turn, each request carries:

- `session_id`: the conversation ID.
- `user`: a SHA-256 hash of the org-scoped user ID, truncated to 32 hex
  characters. The raw user ID and email are never sent.

Background calls made outside a turn send neither field.

## Prompt caching

The model's author is the slug prefix before `/`. Only `anthropic`,
`google` and `qwen` models need explicit breakpoints. For them the
provider sends the system prompt as a text part with
`cache_control: {"type": "ephemeral"}`. Anthropic models add
`"ttl": "1h"` when `ttl = "1h"`; Google and Qwen caches always last 5
minutes, so they get no `ttl`. Anthropic models also get the breakpoint on the last part
of the last user message. Other models get no `cache_control`, because
their upstreams cache automatically.

Every request in a conversation carries the same `session_id` (see
[Request identity](#request-identity)), which keeps it on the same
upstream so the cache can hit.

Usage reports cache reads as cached input tokens, from
`prompt_tokens_details.cached_tokens`, and cache writes as cache-write
input tokens, from `prompt_tokens_details.cache_write_tokens`.

## Errors and retries

Responses with status 408, 429, 500, 502, 503, 504, 524 or 529 are
retried with exponential backoff, up to 5 retries by default. When
OpenRouter sends `Retry-After`, the wait is at least that long. A
`Retry-After` above the 60-second retry cap ends retries at once.

Status 400, 401, 402, 403, 404 and 413 fail without a retry.

Error messages include the HTTP status and, when present,
OpenRouter's `metadata.error_type`. A 200 response that carries only an
`error` object also fails.

When streaming, the provider ignores keep-alive comments. An error in the
middle of a stream fails the call. A stream that ends without `[DONE]` or
a finish reason fails instead of returning partial text.

## Reasoning

### Output

Reasoning entries of type `reasoning.text` and `reasoning.summary` are
shown as thinking. Encrypted entries are never shown. A response without
`reasoning_details` falls back to its plain `reasoning` text. With
`exclude = true`, no thinking is shown.

- Streaming: thinking is streamed as it arrives.
- Non-streaming, tool calls: the thinking is attached to the tool calls.
- Non-streaming, final answer: the thinking is dropped, because a final
  answer has no slot for it.

A response that stops at the token limit (`finish_reason: "length"`)
with no answer text and no tool call fails. See
[Troubleshooting](#openrouter-stopped-at-the-token-budget-before-answering).

### Continuity

When a tool-call response carries `reasoning_details`, they are stored
unchanged with that assistant message, including encrypted entries and
signatures. Streamed pieces are merged by their `index`. Later requests
in the conversation send the details back on that message, in their
original order. This holds after a server restart, because history is
rebuilt from the database.

Details are sent back only when they were stored by OpenRouter for the
model configured now. Switching provider or model drops them.

## Authentication

Obtain an API key from [openrouter.ai/keys](https://openrouter.ai/keys)
and set it via environment variable or config file:

```sh
# Environment variable (recommended — avoids secrets in config files)
export OPENROUTER_API_KEY=sk-or-...

# Or in org.toml
[llm]
provider = "openrouter"
api_key  = "sk-or-..."
```

The provider checks `api_key` in the config first, then falls back to
the `OPENROUTER_API_KEY` environment variable.

## Capabilities

The provider reads what the configured model can do from OpenRouter's
model catalog. When the provider is built, it fetches `{base_url}/models`
and `{base_url}/models/{model}/endpoints` in the background. The first
chat call waits for that fetch for at most 5 seconds.

| Capability     | Value                                                                                                                                                                                                                              |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tool calling   | Native when the model lists `tools` in `supported_parameters`, else none                                                                                                                                                           |
| Streaming      | Yes                                                                                                                                                                                                                                |
| Vision         | Yes when the model lists `image` in its input modalities; otherwise each image is replaced by a short text note                                                                                                                    |
| PDF input      | Native when the model lists `file` in its input modalities; otherwise parsed by the `file-parser` plugin. See [PDFs](#pdfs)                                                                                                        |
| Audio input    | Sent as audio when `audio_input.enabled` and the model lists `audio` in its input modalities. See [Voice](#voice)                                                                                                                  |
| Audio output   | Spoken replies when `audio_output.enabled` and the model lists `audio` in its output modalities. See [Voice](#voice)                                                                                                               |
| Context window | Smallest `context_length` among the endpoints routing can select                                                                                                                                                                   |
| Chat options   | `tool_choice`, `parallel_tool_calls` and JSON schemas are sent natively when the model lists them in `supported_parameters`, with `provider.require_parameters = true`; otherwise `tool_choice` falls back to a prompt instruction |
| Embeddings     | Yes, through OpenRouter's embeddings endpoint                                                                                                                                                                                      |

History compaction reads the context window on every turn.
`[llm.openrouter.provider]` decides which endpoints count:

- `ignore` removes the named providers.
- `only` keeps only the named providers.
- `order` keeps only the named providers when `allow_fallbacks = false`.

A slug names an endpoint when it equals the endpoint's tag, is the start
of a tag variant (`deepinfra` matches `deepinfra/fp8`), or equals its
provider name. Without endpoint data, the model's own `context_length`
is used.

Until the catalog loads, or when it can't be reached, the provider
reports native tools, no vision, and an unknown context window. A failed
fetch is retried on a chat call, no sooner than 10 minutes later. Once a
fetch succeeds, the catalog is not fetched again for the life of the
provider.

## Embeddings

With `[llm] provider = "openrouter"` and no `[llm.embeddings]` section,
vector search embeds through OpenRouter's `/embeddings` endpoint, using
the chat key and `base_url`. The model is `[llm] embedding_model`, or
`openai/text-embedding-3-small` when it is unset.

To embed through OpenRouter while chatting with any provider, set
`[llm.embeddings] provider = "openrouter"`:

```toml
[llm.embeddings]
provider = "openrouter"
model    = "openai/text-embedding-3-small" # default
# base_url = "https://openrouter.ai/api/v1" # default
# api_key  = "sk-or-..."

[llm.embeddings.openrouter.provider]
data_collection = "deny"
```

The key is `[llm.embeddings] api_key`, else the `[llm]` key when
`[llm] provider = "openrouter"`, else `OPENROUTER_API_KEY`.
`[llm.embeddings.openrouter.provider]` takes the same keys as
`[llm.openrouter.provider]` and is sent as the request's `provider`.
Chat routing does not apply to embeddings.

Each request posts `model`, `input` and `encoding_format = "float"`.
More than 2048 texts are split across requests. Vectors come back in
input order, and a response with the wrong number of vectors is an
error. Errors and retries work as for chat, and errors name the model.

## Transcription

`[transcription] provider = "openrouter"` transcribes voice messages through OpenRouter's `/audio/transcriptions` endpoint, by default with `openai/whisper-1`. The key is `[transcription] api_key`, else the `[llm]` key when `[llm] provider = "openrouter"`, else `OPENROUTER_API_KEY`. See [Voice Transcription](transcription.md#openrouter).

## Text-to-speech

`[tts] provider = "openrouter"` speaks replies through OpenRouter's `/audio/speech` endpoint, by default with `openai/gpt-4o-mini-tts` and voice `alloy`. The key resolves as for transcription. At startup the provider checks the voice, `speed` and `instructions` against the model's catalog entry. See [Text-to-speech](transcription.md#openrouter-tts).

## Voice

A voice turn is a message that carries a recording. Every interface that
accepts voice passes the recording into the turn, with the transcript as
the message text, or `[Voice message]` when there is no transcript. See
[Voice Transcription](transcription.md#how-it-works).

The recording reaches the model only on the turn it arrives. History
never replays audio. A provider that doesn't take audio, including
OpenRouter without `audio_input`, gets the text alone.

```toml
[llm]
provider = "openrouter"
model    = "<slug of a model with audio input and output>"

[llm.openrouter.audio_input]
enabled = true

[llm.openrouter.audio_output]
enabled = true
voice   = "alloy"
```

### Audio input

Audio input is active when `audio_input.enabled` is true and the model's
catalog entry lists `audio` in its input modalities (see
[Capabilities](#capabilities)).

The recording goes in the user message as an `input_audio` part. A format
missing from `formats` is converted to 16 kHz mono WAV with ffmpeg. A
recording longer than `max_secs`, or one that ffprobe can't read or ffmpeg
can't convert, is left out, and the text is sent alone. Both tools must be
on `PATH`.

With audio attached, the text part is prefixed with
`Transcript (may contain errors): `, unless it is just `[Voice message]`.

If OpenRouter answers 400 with a message that mentions audio or
modalities, the call is repeated once without audio. Audio input then
stays off until restart. The reason is logged once. See
[Troubleshooting](#openrouter-rejected-audio-input-sending-voice-notes-as-text-from-now-on).

### Spoken replies

On a voice turn, when `audio_output.enabled` is true and the model's
catalog entry lists `audio` in its output modalities, the call streams
with `modalities` including `audio` and
`audio: {"voice": ..., "format": "pcm16"}`.

The PCM is transcoded with ffmpeg to the `[tts] format` (`mp3` by
default; `wav` and `ogg` also work; `pcm` is passed through as is) and
attached to the reply as an audio clip. When the model sends no text, the
transcript of its speech becomes the answer text. If the transcode fails,
the answer is text only.

If OpenRouter answers 400 with a message that mentions audio or
modalities, the call is repeated once without audio. Spoken replies then
stay off until restart. See
[Troubleshooting](#openrouter-rejected-audio-output-replying-in-text-from-now-on).

### Fallback

Any provider can speak a voice turn's answer. When `[tts]` is
configured and the turn produced no audio, neither from the model nor from
a tool such as `voice-response`, the orchestrator speaks the final answer
with the TTS provider and attaches the clip. See
[Text-to-speech](transcription.md#text-to-speech).

Without `[tts]`, voice turns reply in text, and so does a turn whose
synthesis fails. Typed turns never get a clip.

## Image generation

`[images] provider = "openrouter"` enables the `image-generate` tool, which creates and edits images through OpenRouter's `/images` endpoint, by default with `openai/gpt-image-2.6-flare`. The key resolves as for transcription. See [Image Generation](images.md).

## Chat image output

Some models, such as Gemini image models (`google/gemini-2.5-flash-image`),
answer with images directly. With `image_output = true`, every chat
request carries `modalities: ["image", "text"]`, plus `image_config` when
you set [`[llm.openrouter.image_config]`](#llmopenrouterimage_config-section).
With `image_output = false`, neither is sent.

```toml
[llm]
provider = "openrouter"
model    = "google/gemini-2.5-flash-image"

[llm.openrouter]
image_output = true

[llm.openrouter.image_config]
aspect_ratio = "16:9"
image_size   = "2K"
```

The provider checks the model's catalog entry (see
[Capabilities](#capabilities)). If `architecture.output_modalities` lacks
`image`, it logs the reason followed by `; chat calls will fail`, and
every chat call fails with that reason without contacting OpenRouter. See
[Troubleshooting](#openrouter-model-slug-does-not-output-images-remove-image_output-from-llmopenrouter).

Images are read from `message.images[].image_url.url`. When streaming,
they are read from `delta.images` and from the `message.images` of the
last chunk. A URL that appears more than once is kept once.

- A data URI is decoded from base64.
- Any other URL is downloaded at once. The MIME type comes from
  `Content-Type`, or is `image/png` when that is missing or not an
  `image/*` type.
- A failed download or invalid base64 fails the call.

The images are attached to the reply as generated media. The
orchestrator stores them as attachments of the assistant message, so
every interface delivers them the way it delivers tool attachments.

This is separate from the `image-generate` tool, which the model calls
and which uses its own `[images]` settings. See
[Image generation](#image-generation).

## Models

OpenRouter provides access to 300+ models. Use the `provider/model`
slug format. Browse the full catalog at
[openrouter.ai/models](https://openrouter.ai/models).

### Popular choices

| Model slug                           | Provider  | Notes                        |
| ------------------------------------ | --------- | ---------------------------- |
| `anthropic/claude-sonnet-4-20250514` | Anthropic | Claude Sonnet 4, recommended |
| `anthropic/claude-opus-4-20250514`   | Anthropic | Claude Opus 4, most capable  |
| `openai/gpt-4o`                      | OpenAI    | GPT-4o                       |
| `openai/o3-mini`                     | OpenAI    | Reasoning model              |
| `google/gemini-2.5-pro-preview`      | Google    | Gemini 2.5 Pro               |
| `meta-llama/llama-3.3-70b-instruct`  | Meta      | Llama 3.3 70B                |
| `mistralai/mistral-large-latest`     | Mistral   | Mistral Large                |

### Free models

OpenRouter offers several free-tier models (rate-limited):

| Model slug                              | Notes             |
| --------------------------------------- | ----------------- |
| `meta-llama/llama-3.3-8b-instruct:free` | Llama 3.3 8B free |
| `mistralai/mistral-7b-instruct:free`    | Mistral 7B free   |
| `google/gemma-2-9b-it:free`             | Gemma 2 9B free   |

## OpenRouter vs direct providers

You can reach Claude and GPT through OpenRouter, but the **direct
providers** (Anthropic, OpenAI) offer provider-specific features that
OpenRouter does not expose:

| Feature                       | Direct | OpenRouter |
| ----------------------------- | ------ | ---------- |
| Tool calling                  | Yes    | Yes        |
| Streaming                     | Yes    | Yes        |
| Vision                        | Yes    | Yes        |
| Anthropic hosted web search   | Yes    | No         |
| Anthropic hosted web fetch    | Yes    | No         |
| OpenAI OAuth (Codex billing)  | Yes    | No         |
| 300+ model selection          | No     | Yes        |
| Single API key for all models | No     | Yes        |

Use OpenRouter when you want model flexibility. Use direct providers
when you need provider-specific features.

## Example configurations

### Minimal

```toml
[llm]
provider = "openrouter"
model    = "anthropic/claude-sonnet-4-20250514"
# api_key set via OPENROUTER_API_KEY env var
```

### Full

```toml
[llm]
provider        = "openrouter"
model           = "anthropic/claude-sonnet-4-20250514"
api_key         = "sk-or-..."
max_iterations  = 80
timeout_secs    = 120

[llm.openrouter]
referer    = "https://my-app.example.com"
title      = "My App"
max_tokens = 8192

[llm.embeddings]
provider = "voyage"
model    = "voyage-3-lite"
```

### Private routing

Route only to providers that neither collect data nor retain it:

```toml
[llm.openrouter.provider]
data_collection = "deny"
zdr             = true
```

### Fallback models

Requests send `models = ["anthropic/claude-sonnet-4-20250514", "openai/gpt-4o"]`:

```toml
[llm]
provider = "openrouter"
model    = "anthropic/claude-sonnet-4-20250514"

[llm.openrouter]
fallback_models = ["openai/gpt-4o"]
```

### Reasoning

```toml
[llm.openrouter.reasoning]
effort = "high"
```

### Web search

```toml
[llm.openrouter.web_search]
enabled         = true
engine          = "exa"
max_results     = 5
exclude_domains = ["example.com"]
```

### Using a free model for testing

```toml
[llm]
provider = "openrouter"
model    = "meta-llama/llama-3.3-8b-instruct:free"
```

## Troubleshooting

### "OpenRouter API key not found"

Set the key via environment variable or config file:

```sh
export OPENROUTER_API_KEY=sk-or-...
```

Or add `api_key = "sk-or-..."` to the `[llm]` section in org.toml.

### Requests timing out

Some models on OpenRouter can have higher latency due to routing.
Increase the timeout:

```toml
[llm]
timeout_secs = 300
```

### "OpenRouter has no model `<slug>`"

The catalog does not list the configured model. The provider logs
`OpenRouter has no such model; chat calls will fail`, and every chat call
fails without sending anything to OpenRouter. The catalog is not fetched
again, so fix the slug and restart.

Check the slug at [openrouter.ai/models](https://openrouter.ai/models).
Slugs use the `provider/model-name` format (e.g.
`anthropic/claude-sonnet-4-20250514`).

### "OpenRouter model catalog unavailable; using conservative capabilities"

The catalog fetch failed. Chat still works, with native tools, no vision,
and an unknown context window. The provider retries on a chat call at least 10
minutes later. Check the network path to OpenRouter and the API key.

If the catalog is only slow, the first chat call logs
`OpenRouter model catalog is slow; continuing without it` after 5
seconds and goes ahead without it.

### Rate limiting (429)

The provider retries a 429, waiting at least as long as `Retry-After`.
If OpenRouter asks for more than 60 seconds, the call fails at once.
Free-tier models have tight request limits; switch to a paid model or
add credits to your OpenRouter account.

### "the OpenRouter account lacks credits"

OpenRouter returned 402. Add credits to your OpenRouter account. The
call is not retried.

### "moderation reasons: ..."

OpenRouter returned 403 because the input was flagged. The message lists
the `metadata.reasons` OpenRouter gave. Change the input or pick a model
without that moderation. The call is not retried.

### "`categories` takes at most two values"

`[llm.openrouter] categories` has more than two entries. Remove the extra
ones.

### "`search_context_size` works only with `engine = "native"`"

`[llm.openrouter.web_search]` sets `search_context_size` with an `engine`
other than `"native"`, so config load fails. Set `engine = "native"`,
remove `engine`, or remove `search_context_size`.

### "OpenRouter stream ended before the response was complete"

The stream closed before `[DONE]` or a finish reason. The partial text is
discarded; send the message again.

### "OpenRouter stopped at the token budget before answering"

The response hit the token limit before any answer or tool call. When
the model reasoned, the message reads:

```text
OpenRouter stopped at the token budget before answering: the budget was used up by reasoning. Raise [llm.openrouter] max_tokens or lower the reasoning effort.
```

Otherwise:

```text
OpenRouter stopped at the token budget before answering. Raise [llm.openrouter] max_tokens.
```

Raise `[llm.openrouter] max_tokens`, or lower `effort` or
`[llm.openrouter.reasoning] max_tokens`.

### "`effort` and `max_tokens` cannot both be set"

`[llm.openrouter.reasoning]` sets both. Keep one.

### "OpenRouter model `<slug>` does not support reasoning effort; remove `effort` from [llm.openrouter.reasoning]"

The model's catalog entry has no effort selection. Remove `effort`, or
pick a reasoning model. Restart after the fix; the catalog is not
fetched again.

### "OpenRouter model `<slug>` does not support reasoning effort `<effort>`; it supports ..."

The model lists `supported_efforts` and `effort` is not one of them.
Set `effort` to one listed in the message, then restart.

### "reasoning is mandatory for OpenRouter model `<slug>`, so effort `none` is not allowed"

The model always reasons. Set another `effort`, then restart.

### "OpenRouter model `<slug>` does not output images; remove `image_output` from [llm.openrouter]"

`image_output = true`, but the model's catalog entry does not list
`image` in its output modalities. Remove `image_output`, or pick an
image-output model such as `google/gemini-2.5-flash-image`. Restart after
the fix; the catalog is not fetched again.

### "OpenRouter rejected audio input; sending voice notes as text from now on"

OpenRouter answered 400 about audio or modalities. The call was repeated
without the recording, and no audio is sent until restart. Pick a model
that lists `audio` in its input modalities, or remove `audio_input`.

### "OpenRouter rejected audio output; replying in text from now on"

OpenRouter answered 400 about audio or modalities. The call was repeated
without asking for speech, and no audio is requested until restart. Pick a
model that lists `audio` in its output modalities, or remove
`audio_output`. With `[tts]` configured, the TTS provider still speaks
voice turns.

### "could not convert the voice note; sending text only"

ffmpeg failed to convert a recording to WAV. The turn went ahead on the
transcript. Check that `ffmpeg` and `ffprobe` are on `PATH`. The same
fallback logs `could not probe the voice note` when ffprobe fails and
`voice note has no readable duration` when it reports no duration.

### "could not transcode OpenRouter's spoken reply"

ffmpeg failed to turn the model's PCM into the `[tts] format`. The reply
is text only. Check that `ffmpeg` is on `PATH`.
