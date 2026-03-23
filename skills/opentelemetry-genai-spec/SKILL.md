---
name: opentelemetry-genai-spec
description: >
  OpenTelemetry semantic conventions for Generative AI attributes. Use when
  recording telemetry for GenAI.
license: MIT
---

# opentelemetry-genai-spec

Use this skill when adding or reviewing OpenTelemetry instrumentation for LLM
systems.

## Goals

- Emit portable `gen_ai.*` telemetry that works across backends.
- Follow OTel semantic convention MUST/SHOULD guidance.
- Avoid leaking prompt, tool, or retrieval sensitive data by default.

## Minimum baseline attributes

Always try to capture these first:

- `gen_ai.provider.name`
- `gen_ai.operation.name`
- `gen_ai.request.model`
- `gen_ai.response.model`
- `gen_ai.response.id`
- `gen_ai.response.finish_reasons`
- `gen_ai.usage.input_tokens`
- `gen_ai.usage.output_tokens`
- `gen_ai.conversation.id`

## Add-by-capability attributes

- **Agent metadata:** `gen_ai.agent.id`, `gen_ai.agent.name`,
  `gen_ai.agent.description`, `gen_ai.agent.version`
- **Prompting/runtime controls:** `gen_ai.request.max_tokens`,
  `gen_ai.request.temperature`, `gen_ai.request.top_p`, `gen_ai.request.top_k`,
  `gen_ai.request.seed`, `gen_ai.request.stop_sequences`,
  `gen_ai.request.choice.count`
- **Structured content:** `gen_ai.input.messages`, `gen_ai.output.messages`,
  `gen_ai.system_instructions`, `gen_ai.output.type`
- **Tool use:** `gen_ai.tool.name`, `gen_ai.tool.type`, `gen_ai.tool.description`,
  `gen_ai.tool.call.id`, `gen_ai.tool.call.arguments`, `gen_ai.tool.call.result`,
  `gen_ai.tool.definitions`
- **RAG/retrieval:** `gen_ai.data_source.id`, `gen_ai.retrieval.query.text`,
  `gen_ai.retrieval.documents`
- **Evaluation:** `gen_ai.evaluation.name`, `gen_ai.evaluation.score.value`,
  `gen_ai.evaluation.score.label`, `gen_ai.evaluation.explanation`

## Normative rules to enforce

- For `gen_ai.input.messages`, `gen_ai.output.messages`,
  `gen_ai.system_instructions`, and `gen_ai.retrieval.documents`, follow the
  linked OTel JSON schemas.
- On **events**, record these payloads in structured form.
- On **spans**, JSON string fallback is allowed only when structured attributes
  are not supported; prefer structured values.
- Keep input messages in original send order.
- Keep output mapping 1 message to 1 generated choice.
- Use well-known enum values when they apply (provider, operation, output type,
  token type).
- Keep `gen_ai.evaluation.score.label` low cardinality.
- Avoid `gen_ai.tool.definitions` by default; enable only for deep debugging.

## Sensitive-data defaults

Treat these as sensitive and opt-in by policy:

- `gen_ai.input.messages`
- `gen_ai.output.messages`
- `gen_ai.system_instructions`
- `gen_ai.tool.call.arguments`
- `gen_ai.tool.call.result`
- `gen_ai.retrieval.query.text`

Implementation guidance:

- Default to metadata-only telemetry in production.
- Add redaction, truncation, or filtering controls before enabling content
  capture.
- Document who can access raw traces and retention settings.

## Deprecation mapping

- `gen_ai.system` -> `gen_ai.provider.name`
- `gen_ai.usage.prompt_tokens` -> `gen_ai.usage.input_tokens`
- `gen_ai.usage.completion_tokens` -> `gen_ai.usage.output_tokens`
- `gen_ai.openai.request.response_format` -> `gen_ai.output.type`
- `gen_ai.openai.request.seed` -> `gen_ai.request.seed`
- `gen_ai.openai.request.service_tier` -> `openai.request.service_tier`
- `gen_ai.openai.response.service_tier` -> `openai.response.service_tier`
- `gen_ai.openai.response.system_fingerprint` ->
  `openai.response.system_fingerprint`

Also deprecated with no direct replacement:

- `gen_ai.prompt`
- `gen_ai.completion`

## Practical rollout checklist

1. Instrument baseline attrs for every LLM call.
2. Add token usage and finish reasons.
3. Add tool and retrieval attrs where those capabilities exist.
4. Gate content attrs behind explicit config.
5. Validate event/span payload encoding rules in tests.
6. Track semconv updates because many GenAI attrs are still Development.
