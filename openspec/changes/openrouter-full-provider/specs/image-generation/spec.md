# Spec Delta

## Purpose

Lets the assistant create and edit images on request through a configurable image-generation provider. Results are delivered to users through the existing image attachment pipeline.

## ADDED Requirements

### Requirement: Image generation provider

`[images]` in `org.toml` SHALL accept the following:

- `provider = "openrouter"`
- `enabled` (default `true`): when it is `false`, the provider is not built and the tool is not offered
- `model` (default `openai/gpt-image-2.6-flare`)
- `base_url`
- `api_key`, resolved per `openrouter-key-reuse`
- `output_format` (default `png`)
- `quality`
- `[images.openrouter.provider]` routing keys

Generation SHALL post to `{base_url}/images` with `response_format: "b64_json"`. When the response nevertheless carries `url` entries, the provider SHALL download each image before the URL expires. The `session_id` and `user` fields SHALL follow `openrouter-chat`.

#### Scenario: Generate one image

- **WHEN** a generation request asks for "a lighthouse at dusk" with aspect ratio `16:9`
- **THEN** the provider posts `{model, prompt, aspect_ratio: "16:9", output_format: "png", response_format: "b64_json"}` and returns PNG bytes with MIME type `image/png`

### Requirement: Validation against the image model catalog

When the provider is built, it SHALL read the model from `{base_url}/images/models`. An unknown model SHALL fail the build. Requests SHALL be checked against the model's `supported_parameters` before they are sent: an enum value outside its list, a range value outside its bounds, or a parameter the model does not list SHALL fail the request with the allowed values, and SHALL NOT call OpenRouter. Reference images SHALL be accepted only when the model lists image input.

#### Scenario: Unsupported resolution

- **WHEN** a request asks for `resolution = "8K"` on a model that lists `["1K", "2K", "4K"]`
- **THEN** the request fails without an upstream call and lists the allowed values

### Requirement: image-generate tool

When `[images]` is configured, the assistant SHALL offer an `image-generate` tool with these parameters:

- `prompt` (required)
- `n` (1 to 4)
- `aspect_ratio`, `resolution`, `quality`, `background`, `seed`
- `reference_attachment_ids`: images from the current conversation, used as edit inputs and sent as `input_references` data URIs, at most 16.

Generated images SHALL be returned as tool attachments, persisted and delivered per `image-outbound`. When the provider returns fewer images than requested, the tool result SHALL report how many were produced. When `[images]` is not configured, the tool SHALL NOT be offered.

#### Scenario: Edit an attached photo

- **WHEN** the user attaches a photo and asks to make the sky purple
- **THEN** the model calls `image-generate` with that attachment's ID, the request carries it as an `input_references` entry, and the edited image is delivered in the reply

#### Scenario: Not configured

- **WHEN** `[images]` is absent
- **THEN** `image-generate` is not offered to the model

### Requirement: Image generation errors and cost

Image generation errors SHALL follow the error and retry rules in `openrouter-chat`. Usage cost SHALL be recorded on the tool span as `assistant.llm.cost_usd`.

#### Scenario: Out of credits

- **WHEN** OpenRouter answers 402
- **THEN** the tool returns an error result saying the account lacks credits, and the turn continues
