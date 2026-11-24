## Why

Tool execution is effectively unguarded. Verified at `1011f60a`:

- No built-in tool overrides `ToolHandler::requires_confirmation` (default
  `false`, `crates/core/src/tool.rs`). Only the MCP bridge and one Slack tool do.
  The gate in `crates/runtime/src/orchestrator/dispatch.rs` additionally needs
  `ctx.interactive` *and* a registered callback, so scheduled, chat and any other
  non-interactive turn bypasses it entirely. Main orchestrator contexts set
  `allowed_tools: None`.
- `file-read`, `file-write` and `file-edit` accept any absolute or `~` path: no
  root, no canonicalization, no symlink check.
- `bash` takes an uncapped `timeout_secs`, buffers all output before capping it,
  and auto-attaches any image path printed to stdout (up to 10 MB each), which
  lets an injected command exfiltrate files through the attachment channel.
- `web-fetch` checks the URL scheme only: no loopback / private / link-local /
  metadata guard, no re-check on redirect. `crates/workflow-http` has the same
  gap.
- `process` sessions never expire and stored lines have no length cap.

Prompt injection through fetched pages, files or MCP output can therefore drive
any of these primitives with no check at all.

## What Changes

- **BREAKING** New tool-permission pipeline evaluated in the orchestrator before
  every tool execution (global executor path and subagent path):
  1. **Static rules** (code defaults + `[permissions]` config): deny / ask /
     allow rules of the form `tool(argument-pattern)`, workspace-root
     confinement for file tools after canonicalization and symlink resolution,
     hard-deny lists for dangerous shell commands and SSRF targets. Static deny
     always wins and never reaches the model.
  2. **Decision model**: calls the static rules leave undecided go to a
     pinned-version Jev decision model. Approve only at or above a configured
     confidence threshold (default `0.9`); fail closed on error, timeout or
     malformed answer.
  3. **Fallback**: interactive turns use the existing human confirmation
     callback; non-interactive turns are denied and the reason is returned to
     the LLM as `ToolOutput::error`.
- Every decision is recorded as a tracing event (decider, rule or model,
  confidence, outcome).
- `[permissions]` section in the runtime config; documented under `docs/` with
  an ADR.
- File tools enforce workspace roots themselves (defence in depth: the MCP
  server and workflows call the executor without the orchestrator).
- `web-fetch` and `workflow-http` get an SSRF guard (resolve, block
  private/loopback/link-local/ULA/metadata ranges, re-check every redirect hop).
- `bash`: capped `timeout_secs`, streaming output cap, image auto-attach
  restricted to workspace roots. `process`: session TTL, session cap, per-line
  cap.
- **No backward compatibility.** Permissive defaults are removed outright: with
  no `[permissions]` config, undecided calls in non-interactive turns are
  denied.

## Capabilities

### New Capabilities

- `tool-permissions`: the decision pipeline, rule grammar, decision model
  contract and audit events.
- `file-tool-confinement`: workspace-root confinement for `file-read`,
  `file-write`, `file-edit`, `file-glob`.
- `outbound-request-guard`: SSRF guard for `web-fetch` and `workflow-http`.
- `execution-limits`: `bash` and `process` resource limits.

### Modified Capabilities

None (no existing spec covers tool execution).

## Impact

- `crates/core`: `permission` module (types, `ToolDecider` trait, rule engine,
  path guard), `PermissionsConfig` in `AssistantConfig`.
- `crates/runtime`: pipeline wiring in `orchestrator/dispatch.rs` and
  `orchestrator/subagent.rs`; decision-model client.
- `crates/tool-executor`: file tools, `web-fetch`, `bash`, `process`.
- `crates/workflow-http`: SSRF guard.
- Docs: `docs/tool-permissions.md`, `docs/adr/adr-0011-tool-permission-decision-model.md`.
- Out of scope: `crates/web-ui` (separate rework in flight); the MCP server's
  direct `executor.execute` path is hardened only by the tool-level guards
  (confinement, SSRF, limits), not by the decision pipeline — tracked as
  follow-up.
