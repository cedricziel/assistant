# Spec Delta

## Purpose

Lets memory and vector search use OpenRouter embedding models, both when OpenRouter is the chat provider and as a dedicated embedding provider. Each provider gets a correct default embedding model.

## ADDED Requirements

### Requirement: Dedicated OpenRouter embedding provider

The system SHALL accept `provider = "openrouter"` in `[llm.embeddings]`. Requests SHALL post `{model, input, encoding_format: "float"}` to `{base_url}/embeddings`. `base_url` SHALL default to `https://openrouter.ai/api/v1`, and `model` SHALL default to `openai/text-embedding-3-small`. `[llm.embeddings.openrouter.provider]` SHALL accept the same routing keys as chat.

#### Scenario: Dedicated provider

- **WHEN** `[llm] provider = "anthropic"` and `[llm.embeddings] provider = "openrouter"` are set and text is embedded
- **THEN** OpenRouter's embeddings endpoint returns the vector

### Requirement: Native embeddings for the OpenRouter chat provider

With `[llm] provider = "openrouter"` and no `[llm.embeddings]`, embeddings SHALL use OpenRouter's embeddings endpoint with the chat provider's key and base URL.

#### Scenario: No dedicated provider

- **WHEN** `[llm] provider = "openrouter"` is set without `[llm.embeddings]` and text is embedded
- **THEN** a vector is returned and no error is raised

### Requirement: Native batching

Embedding a batch SHALL send all texts in one request as an `input` array. Results SHALL be returned in input order, ordered by `data[].index`. A response with a different number of embeddings than inputs SHALL be an error. Batches larger than 2048 texts SHALL be split into consecutive requests.

#### Scenario: Batch of three

- **WHEN** three texts are embedded as a batch
- **THEN** one request carries a three-element `input`, and the vectors come back in input order even when `data` is returned out of order

### Requirement: Provider-specific default embedding model

`[llm] embedding_model` SHALL be optional. When it is unset, each chat provider SHALL use its own default: `nomic-embed-text` for Ollama, `text-embedding-3-small` for OpenAI and `openai/text-embedding-3-small` for OpenRouter. When it is set, the configured value SHALL be used.

#### Scenario: OpenAI without an embedding model

- **WHEN** `[llm] provider = "openai"` is set without `embedding_model`
- **THEN** embedding requests use `text-embedding-3-small`

### Requirement: Embedding errors

OpenRouter embedding errors SHALL follow the same parsing and retry rules as chat (`openrouter-chat`).

#### Scenario: Unknown model

- **WHEN** OpenRouter answers 404 for the embedding model
- **THEN** the error names the model
