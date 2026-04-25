# Glossary

Canonical domain terms for this repository.

## Persona

Long-lived assistant context with its own identity and memory bundle (for example `SOUL.md`, `IDENTITY.md`, and `USER.md`), plus its own workspace and scoped data.

Use when referring to the assistant's durable context.

## Subagent Process

Ephemeral delegated execution unit spawned by a Persona to complete a task.

Use when referring to delegated task execution, including `agent-spawn`, `agent-status`, and `agent-terminate` runtime behavior.

## A2A Profile

Persona-attached external protocol contract used for discovery and interaction (agent card metadata, capabilities, interfaces, and auth requirements).

Use when referring to A2A discovery and machine-to-machine integration surfaces.

## Organization (Org)

Top-level tenant containing users, spaces, and shared resources. Each org has its own database (`org.db`), auth configuration, and resource catalog.

## Space

Isolated workgroup within an organization. Scopes personas, conversations, skills, and interface bindings. Each space has its own database (`space.db`) for hard data isolation.

## AuthContext

Runtime context object carrying user identity, org membership, space roles, and scopes through the request/execution stack. Built from JWT tokens or API keys by the auth middleware.

## Role

Position-based access level within the multi-user model. Four predefined roles: `OrgAdmin`, `SpaceAdmin`, `Member`, `Viewer`.

## Scope

Fine-grained permission in `resource:action` format (e.g. `conversations:write`, `personas:read`). Used by API keys to restrict access beyond what the user's role grants.

## Naming Rules

- Prefer explicit terms: `Persona`, `Subagent Process`, `A2A Profile`.
- Avoid unqualified `agent` in architecture and UX docs.
- If a code identifier still uses legacy naming, keep the identifier literal but explain it using canonical terminology.

## Legacy-to-Canonical Mapping

| Legacy phrase       | Canonical phrase    |
| ------------------- | ------------------- |
| assistant context   | Persona             |
| sub-agent/subagent  | Subagent Process    |
| A2A agent/card root | Persona A2A Profile |
