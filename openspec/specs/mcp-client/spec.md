# mcp-client Specification

## Purpose
Define how the assistant behaves as an MCP client toward configured and integration MCP servers.

## Requirements

### Requirement: Protocol negotiation
The client SHALL use revision `2026-07-28` with servers that support it and fall back to `2025-11-25` through `initialize` with servers that do not.

#### Scenario: Initialize-era server
- **WHEN** an external server only supports `2025-11-25`
- **THEN** the client connects and lists its tools

### Requirement: Declared client capabilities
The client SHALL declare the Tasks, Skills and MCP Apps extensions and SHALL NOT declare sampling, roots or elicitation.

#### Scenario: Capabilities sent
- **WHEN** the client sends any request to a `2026-07-28` server
- **THEN** its client capabilities list the three extensions and no sampling, roots or elicitation

### Requirement: Tool list freshness
The client SHALL refresh a server's tools when the server signals a tool list change, so the next turn sees the new list.

#### Scenario: Server adds a tool
- **WHEN** a connected server signals that its tool list changed
- **THEN** the next turn's tool set contains the new tool

### Requirement: Content conversion
Tool results SHALL be converted without silently dropping content: text joins the output text; images and audio become attachments; resource links and embedded text resources are rendered into the output text with their URI; embedded binary resources become attachments; `structuredContent` is passed to the Persona when no text content describes it.

#### Scenario: Audio content
- **WHEN** a tool result contains an audio block
- **THEN** the tool output carries an audio attachment with the block's MIME type

#### Scenario: Structured content only
- **WHEN** a tool result has `structuredContent` and no text blocks
- **THEN** the tool output text is the JSON serialization of the structured content

### Requirement: Model-visible tools
Tools whose MCP Apps visibility excludes `model` SHALL NOT be offered to the Persona.

#### Scenario: App-only tool
- **WHEN** an external tool declares visibility `["app"]`
- **THEN** it is absent from the Persona's tool list
