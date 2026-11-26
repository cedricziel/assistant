# Image Generation

The `image-generate` tool creates images from a text prompt, or edits images the user attached. The images come back as attachments of the reply.

## Supported providers

| Provider     | Type   | Default model                | API key env var      |
| ------------ | ------ | ---------------------------- | -------------------- |
| `openrouter` | Hosted | `openai/gpt-image-2.6-flare` | `OPENROUTER_API_KEY` |

## Configuration

`[images]` is set per org. Add to `~/.assistant/orgs/default/org.toml`:

```toml
[images]
provider = "openrouter"
# enabled = true                          # false: no provider, no tool
# model = "openai/gpt-image-2.6-flare"
# base_url = "https://openrouter.ai/api/v1"
# api_key = "sk-or-..."                   # else the [llm] key, else OPENROUTER_API_KEY
# output_format = "png"                   # "png" | "jpeg" | "webp"
# quality = "high"                        # used when a request sets none

# [images.openrouter.provider]
# data_collection = "deny"
```

| Key             | Default                        | Meaning                                        |
| --------------- | ------------------------------ | ---------------------------------------------- |
| `provider`      | _(required)_                   | Only `"openrouter"`.                           |
| `enabled`       | `true`                         | `false` builds no provider and offers no tool. |
| `model`         | `openai/gpt-image-2.6-flare`   | Image model slug.                              |
| `base_url`      | `https://openrouter.ai/api/v1` | API root.                                      |
| `api_key`       | _(see below)_                  | OpenRouter key.                                |
| `output_format` | `png`                          | `png`, `jpeg` or `webp`.                       |
| `quality`       | _(none)_                       | `quality` for requests that don't set one.     |

The key is `[images] api_key`, else the `[llm]` key when `[llm] provider = "openrouter"`, else `OPENROUTER_API_KEY`.

`[images.openrouter.provider]` sets provider routing and takes the same keys as [`[llm.openrouter.provider]`](openrouter.md#llmopenrouterprovider-section).

Unknown keys in `[images]` or `[images.openrouter]` fail config load.

## Startup

When a space starts, the provider reads the configured model from `{base_url}/images/models`:

- If the model isn't listed, the error is logged and `image-generate` is not offered. The space still starts. A missing API key is handled the same way.
- If the catalog can't be reached, a warning is logged and requests are sent without checking them.

Without `[images]`, or with `enabled = false`, the tool is not offered.

## The `image-generate` tool

| Parameter                  | Type     | Meaning                                                        |
| -------------------------- | -------- | -------------------------------------------------------------- |
| `prompt`                   | string   | Required. What to draw, or how to change the reference images. |
| `n`                        | integer  | Number of images, 1 to 4. Default 1.                           |
| `aspect_ratio`             | string   | For example `1:1`, `16:9`.                                     |
| `resolution`               | string   | For example `1K`, `2K`, `4K`.                                  |
| `quality`                  | string   | For example `low`, `medium`, `high`.                           |
| `background`               | string   | `auto`, `transparent` or `opaque`.                             |
| `seed`                     | integer  | Seed for repeatable results.                                   |
| `reference_attachment_ids` | string[] | Up to 16 attachment IDs to edit or use as references.          |

References must be images attached in the same conversation. The tool rejects more than 16 references, unknown IDs, attachments from other conversations, attachments that aren't images, and any references when the model doesn't take image input.

When the catalog was read, each request is checked against the model's entry before it is sent. A value the model doesn't accept, or a parameter it doesn't list, fails the request with the allowed values, and OpenRouter is not called.

The images are named `image-1.png`, `image-2.png` and so on, by the returned type. If fewer images come back than requested, the tool says how many it got.

The permission rules allow `image-generate` without confirmation by default.

## Requests

Each request posts JSON to `{base_url}/images` with `model`, `prompt`, `output_format`, `response_format: "b64_json"`, the set parameters, references as `input_references` data URLs, and the routing as `provider`. During a conversation turn it also carries `session_id` and `user`, as chat does (see [Request identity](openrouter.md#request-identity)).

A result returned as a signed `url` instead of base64 is downloaded at once, because the URL expires.

Errors and retries work as for chat (see [Errors and retries](openrouter.md#errors-and-retries)): a 402 reports that the account lacks credits, and a 429 honors `Retry-After`.

The cost OpenRouter reports in `usage.cost` is recorded on the tool call's span as `assistant.llm.cost_usd`.
