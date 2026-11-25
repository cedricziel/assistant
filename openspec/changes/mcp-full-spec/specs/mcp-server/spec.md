# Spec Delta

## Purpose
Define how the assistant behaves as an MCP server: which protocol revisions it speaks, which features it offers, and how stdio serving works.

## ADDED Requirements

### Requirement: Protocol revisions
The server SHALL speak MCP revision `2026-07-28` and SHALL also accept clients that negotiate `2025-11-25` through `initialize`. A per-request revision other than these SHALL fail with the protocol's unsupported-version error, listing the supported revisions. An `initialize` that proposes an unsupported revision SHALL be answered with a supported one, as the MCP lifecycle requires, and never with the unsupported revision.

#### Scenario: Stateless client discovers the server
- **WHEN** a client sends `server/discover` with protocol version `2026-07-28` in `_meta`
- **THEN** the result lists both supported revisions, the server capabilities and server info

#### Scenario: Initialize-era client connects
- **WHEN** a client sends `initialize` with protocol version `2025-11-25`
- **THEN** the server completes the handshake at `2025-11-25` and serves tools and resources to that client

#### Scenario: Unknown per-request revision
- **WHEN** a request carries protocol version `2024-11-05` in `_meta`, as `2026-07-28` clients do
- **THEN** the server rejects it with the unsupported-version error listing `2026-07-28` and `2025-11-25`

#### Scenario: Initialize proposes an unknown revision
- **WHEN** a client sends `initialize` with protocol version `2024-11-05`
- **THEN** the server answers with protocol version `2025-11-25`, and the client decides whether to continue

### Requirement: Caller identity
Every `tools/call` SHALL run as the identity the request carries, bound to one space: the space named by the `x-space-id` header, or the caller's only space. The caller SHALL hold a role in that space or be an org admin, and a viewer SHALL NOT run tools. A request that carries no identity SHALL be rejected and SHALL NOT run as the system identity. Stdio callers carry the local system identity.

#### Scenario: Tool call runs as the caller
- **WHEN** a client holding the member role in one space calls a tool
- **THEN** the tool runs with that user, org and space in its execution context

#### Scenario: Token scopes do not gate tools
- **WHEN** a member whose token carries only the `mcp:execute` scope calls a tool that the permission policy allows
- **THEN** the tool runs

#### Scenario: No identity
- **WHEN** a request reaches the handler with no caller identity
- **THEN** the call fails with an invalid-request error and no tool or turn runs

#### Scenario: Viewer
- **WHEN** a client holding the viewer role calls a tool
- **THEN** the result has `isError: true` and no tool runs

### Requirement: Tools
The server SHALL list the caller's available assistant tools plus `run-prompt` and `install-skill` in a deterministic order, each with an input schema, and SHALL execute `tools/call` under the permission policy that applies to the caller. A policy refusal SHALL be a tool result with `isError: true`, not a protocol error.

#### Scenario: Tool list is stable
- **WHEN** a client lists tools twice without the tool set changing
- **THEN** both lists contain the same tools in the same order

#### Scenario: Confirmation-required tool from a non-interactive caller
- **WHEN** a client calls `install-skill` and the policy requires confirmation
- **THEN** the result has `isError: true` and explains the refusal

### Requirement: Skill resources
The server SHALL expose each installed skill's `SKILL.md` and bundled files as resources, and a resource template for skill files. Reading a URI that resolves outside the skill's directory SHALL fail with resource-not-found and SHALL NOT read the file.

#### Scenario: Read a bundled script
- **WHEN** a client reads `skills://example/scripts/run.sh`
- **THEN** the result contains that file's content with its MIME type

#### Scenario: Traversal attempt
- **WHEN** a client reads `skills://example/scripts/../../../secret`
- **THEN** the server returns resource-not-found and no file outside the skill directory is opened

### Requirement: Prompts and completions
The server SHALL expose each installed skill as a prompt taking one optional free-text `input` argument, and SHALL answer `completion/complete` for prompt names and skill-resource template arguments with at most 100 values.

#### Scenario: Get a skill prompt
- **WHEN** a client calls `prompts/get` with a skill name
- **THEN** the result contains that skill's instructions, followed by the `input` text when given, as a user message

#### Scenario: Complete a skill name
- **WHEN** a client requests completion for the `name` argument of the skill template with prefix `ex`
- **THEN** the result lists installed skill names starting with `ex`

### Requirement: Change notifications
The server SHALL advertise list-changed support for tools, resources and prompts and SHALL notify subscribed clients when installed skills change. Under `2026-07-28` notifications SHALL be delivered only on a `subscriptions/listen` stream the client opened for that notification kind.

#### Scenario: Skill installed while subscribed
- **WHEN** a client listens for tool and prompt list changes and a skill is installed
- **THEN** the client receives list-changed notifications carrying its subscription id

### Requirement: Progress and cancellation
The server SHALL report progress for `run-prompt` when the request carries a progress token, and SHALL stop the underlying turn when the request is cancelled.

#### Scenario: Cancelled prompt
- **WHEN** a client cancels an in-flight `run-prompt`
- **THEN** the turn stops and no further messages are sent for that request

### Requirement: Stdio serving
`assistant mcp` SHALL serve MCP over stdio, writing only protocol messages to stdout and all logs to stderr. Stdio callers act as the local system identity.

#### Scenario: Logs do not corrupt the stream
- **WHEN** the server logs while handling a request over stdio
- **THEN** stdout contains only well-formed protocol messages
