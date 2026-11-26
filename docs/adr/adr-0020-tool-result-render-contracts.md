# ADR 0020: Tool Result Render Contracts

**Status**: Accepted
**Date**: 2026-11-26
**Builds on**: [ADR-0009](adr-0009-testability-architecture.md)
**Related**: [ADR-0019](adr-0019-generative-ui-a2ui.md)

## Context

A tool result reaches the client as a text blob. The web chat renders that blob
in a generic view, so a shell command, a list of search results, a file diff or
a plan all look alike. We want dedicated views, and we want third-party MCP
servers to get them too. MCP tool names (`mcp__{server}__{tool}`) are not known
ahead of time, so a client cannot choose a view from the tool name. The Rust
server and the Dart client must also agree on every shape without a shared
code generator.

## Decision

**A contract directory is the source of truth.** `schemas/tool-results/` holds
one JSON Schema (2020-12) per contract, `{kind}.schema.json`, and fixtures under
`fixtures/{kind}/`. Every `valid-*.json` fixture must pass the schema and parse
into the model; every `invalid-*.json` fixture must fail the schema. Rust
(`crates/core/tests/tool_render_contracts.rs`) and Dart check the same files.
The seven contracts are `terminal`, `sources`, `link_preview`, `code_file`,
`code_diff`, `data_table` and `plan`.

**The `kind` field tags the payload.** `ToolOutput.data` is a JSON object with a
string `kind`. `ToolRenderData` in `assistant-core` is an enum serialized with
`kind` as the tag. Clients choose a renderer from `kind`, never from the tool
name. Data without a `kind` keeps flowing and shows in a generic JSON view.

**No versioning.** Contracts evolve in place. Top-level `additionalProperties`
is `true`, so adding optional fields is safe. A client that cannot parse a
payload falls back to its generic view and never fails the event. Breaking
changes use a new `kind`.

**Drop, do not truncate.** Dispatch caps serialized `data` at 32 KiB
(`TOOL_RESULT_DATA_LIMIT_BYTES`). Over the cap the whole value is dropped and
the text result stays. Built-in producers keep their own render budgets, cut on
a character boundary and set the contract's truncation flag.

**MCP servers opt in.** An MCP tool that returns `structuredContent` with a
`kind` from this directory gets the same view. The client's parse check is the
safety boundary.

**Data is for clients only.** It never enters the LLM context.

### Related rules

- **A non-zero `bash` exit is a failure.** Dispatch maps `ToolOutput.success =
  false` to the wire status `error`, and `bash` returns `success = false` for a
  non-zero exit. The observation text for the model and the `terminal` data
  (`exit_code = n`) stay the same. The timeline no longer shows a green row for a
  failed command. Commands where non-zero is an expected answer (`grep` with no
  match) also read as errors.
- **Subagent frames carry no data.** `subagent_tool_result` frames never include
  `data`, and nothing extra is persisted for subagent tool output. Subagent
  timelines render inner tool calls as compact rows.

## Consequences

- Adding a contract means fixtures, a schema, an enum variant and a Dart
  renderer. The contract test fails until the first three exist.
- Third-party MCP servers build against these shapes. The no-version policy
  binds us to additive changes.
- Webhook `tool.result` consumers see `error` for tool-level failures and
  non-zero `bash` exits.

## Alternatives considered

- **Pick the view from the tool name.** Rejected: MCP names are unknown.
- **Versioned contracts (`terminal.v2`).** Rejected: the parse-or-fallback rule
  already protects old clients, and versions add negotiation for little gain.
- **Truncate oversize data.** Rejected: a cut JSON value does not parse.
