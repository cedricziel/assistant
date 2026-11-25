# Spec Delta

## Purpose

Makes JSON answers that must match a schema reliable on every provider. Native structured output is used where available, with forced tool calls and validated prompting as fallbacks. Internal judges stop parsing free-form text.

## ADDED Requirements

### Requirement: Schema-conformant JSON from any provider

A structured call SHALL return a JSON value that validates against the given schema. Each provider SHALL use the first strategy it supports:

1. Native JSON-schema response format, with strict mode on.
2. A forced call to a single synthetic tool whose parameters are the schema. The tool call's arguments are the result.
3. The schema embedded in the system prompt, with markdown code fences stripped from the answer before parsing.

The result SHALL be validated against the schema with a JSON Schema validator. On a parse or validation failure, the provider SHALL retry once with the validation errors added as a user message. If the retry also fails, the call SHALL fail with the validation errors and the raw output.

#### Scenario: OpenRouter model with structured outputs

- **WHEN** a structured call goes to a model whose catalog lists `structured_outputs`
- **THEN** the request carries `response_format: {"type": "json_schema", "json_schema": {"name", "strict": true, "schema"}}`, and the parsed content is returned

#### Scenario: Provider with tools but no response format

- **WHEN** the provider supports forced tool choice but not JSON schemas
- **THEN** the request carries one tool built from the schema and forces it, and the tool arguments are returned

#### Scenario: Invalid first answer

- **WHEN** the first answer misses a required property
- **THEN** one retry is made that includes the validation error, and a valid second answer is returned

#### Scenario: Persistent failure

- **WHEN** both attempts fail validation
- **THEN** the call fails with an error that contains the validation errors and the raw output

### Requirement: Response healing on OpenRouter

For non-streaming OpenRouter calls that use a native JSON-schema response format, the request SHALL include `plugins: [{"id": "response-healing"}]` alongside any other plugins.

#### Scenario: Healing plugin

- **WHEN** a structured call goes natively through OpenRouter
- **THEN** the request includes the `response-healing` plugin

### Requirement: Internal judges use structured output

The skill-learner and skill-improver judges SHALL request their decisions as structured output with explicit schemas, instead of parsing free-form text.

#### Scenario: Skill learner decision

- **WHEN** the skill learner evaluates a turn
- **THEN** it receives a schema-valid decision object, and it no longer fails on fenced or chatty JSON
