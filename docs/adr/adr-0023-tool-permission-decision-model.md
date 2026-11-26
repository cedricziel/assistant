# ADR 0023: Tool Permissions With Static Rules and a Pinned Decision Model

**Status**: Accepted
**Date**: 2026-11-25

## Context

The orchestrator runs a tool call the LLM chose with almost no check.

- No built-in tool overrides `ToolHandler::requires_confirmation`, which defaults to `false`. Only the MCP bridge and one Slack tool do.
- The confirmation gate in `crates/runtime/src/orchestrator/dispatch.rs` needs `ctx.interactive` and a registered callback. Scheduled, messenger and Subagent Process turns skip it.
- `file-read`, `file-write` and `file-edit` accept any absolute or `~` path.

Prompt injection through a fetched page, a file or MCP output can therefore drive `bash`, the file tools and `web-fetch` with no check at all.

We want the model of Claude Code permissions (deny, ask and allow rules) plus an automatic decision for the long tail of calls no rule covers. The automatic part is modelled on the Jev decision model from TypeSafe AI.

Facts about Jev that shape the design:

- It is served at `POST https://openrouter.ai/api/alpha/decisions` with model id `typesafe/jev-<version>`, for example `typesafe/jev-1.13`.
- The request is `{model, state, questions}`. Each question gets a probability from 0 to 1. The response has no approve or deny field. The caller turns scores into a verdict.
- A `jev-latest` alias exists and moves when a new version ships. Vendors advise pinning.
- Adversarial text in `state` can shift a score. Vendors say a Jev guard belongs next to deterministic checks, and that the static risk list is the security boundary, not the threshold.

## Decision

Every tool call goes through one pipeline in `assistant-core`, evaluated in the orchestrator before the tool runs. This covers the global executor path and the Subagent Process loop.

1. **Static rules.** Built-in and configured deny, ask and allow rules, and workspace-root confinement for file tools. A static deny always wins and never reaches a model.
2. **Pinned decision model.** Calls the static rules leave open go to Jev. The call is approved only when every question scores at or above a threshold, default `0.9`. A low score, an error, a timeout or a malformed answer means no approval. The model can approve or abstain. It cannot deny, because the scores answer "is this fine?" and not "is this malicious?". Denial stays with step 1.
3. **Fallback.** On an interactive turn the existing human confirmation callback decides. On every other turn the call is denied and the reason goes back to the LLM as the tool result.

### Pipeline order

1. Deny rules, built-in then configured.
2. Confinement of file tools to the workspace roots.
3. Ask rules. A match forces the human path, even if the model would approve.
4. Allow rules, built-in then configured.
5. No rule matched and the tool requires confirmation: human path.
6. No rule matched: decision model, if configured.
7. No model configured: human path.

`PermissionPolicy` in `assistant-core` runs steps 1 to 7 and returns an `Outcome` of `Allow`, `Deny` or `Ask`. The runtime resolves `Ask` with the callback or a denial. This keeps UI callbacks out of core and lets tests drive the pipeline with a scripted `ToolDecider`.

### Rules

A rule is `tool` or `tool(pattern)`. `*` and `**` match any run of characters, including `/`. `\*` is a literal star. `<workspace>` expands to the workspace roots and a leading `~/` to the home directory. The pattern matches the tool's primary argument: `command` for `bash`, `path` for the file tools, the literal prefix of `pattern` for `file-glob`, `url` for `web-fetch`, `query` for `web-search`, and the compact JSON of the parameters for any other tool.

Matching is conservative in both directions. Allow rules for `bash` never match a command containing shell metacharacters. Deny and ask rules match the whole command and every segment split at those characters. File paths are resolved, with `..` applied and symlinks followed, before they are matched or confined.

Configuration lives in `[permissions]` in `config.toml`. See `docs/tool-permissions.md` for the user guide. The decision model adds a `[permissions.decision_model]` table with `enabled`, `model`, `threshold`, `timeout_secs`, `base_url` and `api_key_env`. `model` must be a pinned version. A value containing `latest`, or one that does not end in a version number, is rejected when the config loads.

### The decision model never sees untrusted output

The model receives the tool call as one command string, the working directory, the agent id, and the first 1,000 characters of the user's latest message. It never receives tool output, fetched content or earlier assistant text. Because a score can still be steered, every deterministic deny sits upstream of it and the threshold is high.

### Audit

Each decision emits one `tracing` event on target `assistant::permission` with the fields `tool`, `decider`, `rule`, `confidence`, `outcome` and `interactive`. Arguments and task text are not logged. For a model decision, `decider` is `decision-model`, `rule` is the model id and `confidence` is the lowest score.

### No compatibility mode

With no `[permissions]` section, calls that no rule allows are denied on non-interactive turns. The implicit "non-interactive turns skip confirmation" behaviour is removed, not shimmed.

## Consequences

**Good**

- One decision point for every tool call, with a recorded decider and rule.
- Deny is deterministic. No model output can override it.
- Common safe work needs no prompt: workspace file access, memory and read-only tools are allowed by default.
- Every failure of the model path ends in the human path or a denial.

**Bad / accepted**

- Scheduled tasks, messenger turns and Subagent Processes that use `bash`, `web-fetch` or `process` fail until the owner adds allow rules. This is intended.
- When the model is enabled, every undecided call pays a network round trip, bounded by `timeout_secs` (default 8). Static allow rules are the fast path.
- The model is steerable. It reduces how often a human is asked. It is not the security boundary.
- The MCP server (`crates/mcp-server`) runs the same policy in its `tools/call` handler. MCP callers cannot be prompted, so undecided calls are refused. It does not go through the orchestrator's gate, so it repeats the policy call there.
- Network destinations are not filtered. Rules match the URL string the LLM wrote, so a hostname that resolves to a sensitive address, or a redirect to one, is not caught by a deny rule on the IP.

## Alternatives considered

**Run the pipeline inside `ToolExecutor::execute`.** This would cover the MCP server too. Rejected. The human fallback and the task text of the turn live in the runtime, and the executor is policy-free by design. Moving them down would couple the executor to UI callbacks.

**Decision model only, no static rules.** Rejected. The scores are steerable by adversarial content in the state, and the vendors say the model belongs next to deterministic checks. A model-only gate has no boundary.

**Let the model deny.** Rejected. Jev scores say whether a call looks fine, not whether it is malicious. A low score is weak evidence of harm, so it routes to the human path instead.

**Use `jev-latest`.** Rejected. The alias moves on release, so approval behaviour would change without a config change. The config requires a pinned version.

**An SSRF deny-list for private, loopback and link-local addresses.** Rejected by the owner. Reaching local and LAN services, such as a Home Assistant instance, is a core use case. Users govern `web-fetch` with their own `web-fetch(<url pattern>)` rules instead.
