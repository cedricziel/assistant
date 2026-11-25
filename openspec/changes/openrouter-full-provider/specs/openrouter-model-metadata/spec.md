# Spec Delta

## Purpose

Derives what the configured OpenRouter model can do from OpenRouter's model catalog instead of hard-coded guesses. Compaction, vision, document handling and tool use then match the real model.

## ADDED Requirements

### Requirement: Capabilities from the model catalog

When the provider is built, it SHALL start fetching the configured model's catalog entry and endpoints from `{base_url}/models`. The first chat call SHALL wait for that fetch for at most 5 seconds.

The fetched entry SHALL determine the reported capabilities:

- `context_window_tokens`: the smallest `context_length` among the endpoints that routing can select. When `[llm.openrouter.provider]` sets `only` or `order` with `allow_fallbacks = false`, only those endpoints count.
- `vision`: whether the model accepts image input.
- native tool support: whether `tools` is in `supported_parameters`.
- reasoning support: per the model's `reasoning` object.
- PDF input support: whether the model accepts file input.

Consumers of the context window SHALL read it each turn, so a value that arrives after startup takes effect on the next turn.

#### Scenario: Context window reaches compaction

- **WHEN** the configured model's selectable endpoints report `context_length` 200000 and 128000
- **THEN** the provider reports `context_window_tokens = 128000`, and the next turn compacts against that limit

#### Scenario: Text-only model

- **WHEN** the catalog lists only text input for the model
- **THEN** the provider reports `vision = false`, and image attachments are not sent as image parts

#### Scenario: Model without tool support

- **WHEN** `tools` is not in the model's `supported_parameters`
- **THEN** the provider reports no native tool support

### Requirement: Unknown or unreachable models

When the configured model is not in the catalog, the provider SHALL log an error at startup. Every chat call SHALL then fail with an error that names the model, and SHALL NOT be sent upstream. When the catalog cannot be reached, the provider SHALL use conservative capabilities: native tools, no vision, no PDF input, unknown context window. It SHALL log a warning and retry the fetch at most once every 10 minutes until a fetch succeeds.

#### Scenario: Typo in the model slug

- **WHEN** `model = "anthropic/claude-sonet-5.5"` is not in the catalog
- **THEN** startup logs that OpenRouter has no such model, and each chat call fails with that message

#### Scenario: Catalog outage

- **WHEN** the catalog request fails at startup
- **THEN** chat works with conservative capabilities, and the capabilities update once a later fetch succeeds
