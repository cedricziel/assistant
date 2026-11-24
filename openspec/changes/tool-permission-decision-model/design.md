## Context

The orchestrator dispatches a model-chosen tool call through
`Orchestrator::dispatch_global_tool` (and an inline copy in the subagent loop).
Confirmation is an optional per-tool flag consulted only for interactive turns.
Tools themselves trust their arguments. We want "Claude Code permissions plus a
decision model that makes automatic decisions", modelled on TypeSafe AI's Jev.

Jev facts that shape the design (OpenRouter cookbook, Vercel guide, VentureBeat):

- It is exposed as `POST https://openrouter.ai/api/alpha/decisions`, model id
  `typesafe/jev-<version>` (e.g. `typesafe/jev-1.13`). The request is
  `{model, state, questions}`; each question gets a 0..1 probability
  (`answers.<key>.noul`). There is no approve/deny field: the *caller* turns
  scores into a verdict.
- A `jev-latest` alias exists and moves on release; vendors advise pinning.
- Adversarial content in `state` can shift the score; vendors say a Jev guard
  belongs *alongside deterministic checks, not instead of them*, and that the
  static risk list is the security boundary, not the threshold.

## Goals / Non-Goals

**Goals:** one decision point for every tool call; deterministic rules that the
model can never override; fail-closed everywhere; auditable decisions; fix the
concrete tool-level gaps listed in the proposal.

**Non-Goals:** per-user policy UI, policy in the org DB, gating the MCP server
entry point, gating extension (interface) tools, sandboxing at the OS level.

## Decisions

### D1. Trait + pipeline live in `assistant-core`

```rust
pub struct PermissionRequest<'a> {
    pub tool_name: &'a str,
    pub params: &'a serde_json::Value,
    pub ctx: &'a ExecutionContext,
    pub requires_confirmation: bool,   // ToolHandler::requires_confirmation
    pub task_context: Option<&'a str>, // latest user message, truncated
}

pub enum Verdict { Allow, Deny, Ask, Abstain }

pub struct Decision {
    pub verdict: Verdict,
    pub decider: &'static str,   // "static-rules" | "decision-model" | ...
    pub rule: Option<String>,    // matched rule text or model id
    pub confidence: Option<f32>,
    pub reason: String,          // shown to the LLM on deny
}

#[async_trait]
pub trait ToolDecider: Send + Sync {
    async fn decide(&self, req: &PermissionRequest<'_>) -> Decision;
}
```

`PermissionPolicy` owns `StaticRules` plus an optional
`Arc<dyn ToolDecider>` (the decision model) and yields a final
`Outcome { Allow, Deny(reason), Ask }`. The runtime turns `Ask` into the human
callback (interactive) or a denial (non-interactive). Keeping `Ask` out of core
keeps core free of UI callbacks and makes the pipeline pure and testable with a
scripted decider.

*Alternative considered:* put the pipeline inside `ToolExecutor::execute` so the
MCP server is covered too. Rejected: the human fallback and turn task context
live in the runtime, and the executor is deliberately policy-free. Tool-level
guards (D4–D6) cover the MCP path instead.

### D2. Evaluation order

1. Static **deny** (config deny rules, built-in hard-deny lists, path outside
   workspace roots for file tools). First match wins → `Deny`. Never reaches the
   model.
2. Static **ask** rules → `Ask` (forces the human path even if the model would
   approve).
3. Static **allow** rules (config + built-in safe defaults, e.g.
   `file-write(<workspace>/**)`, read-only memory/list tools) → `Allow`.
4. No rule matched and `requires_confirmation` → `Ask`.
5. No rule matched → decision model, if configured: score ≥ threshold on every
   question → `Allow`; otherwise (low score, error, timeout, malformed) →
   `Ask`. The model can approve or decline to approve; it cannot deny, because
   Jev scores are calibrated "is this fine?", not "is this malicious?".
   Denial is the job of step 1.
6. No model configured → `Ask`.

`Ask` is resolved by the runtime: interactive + callback → human decides;
otherwise → deny with reason
`"'<tool>' needs approval but this turn cannot prompt a user (no allow rule matched)"`.

### D3. Rule grammar

`tool-name` or `tool-name(pattern)`. `*` (and `**`) match any run of
characters including `/`. The pattern is matched against the tool's *primary
argument*: `bash`→`command`, `file-*`→`path`, `web-fetch`→`url`,
`web-search`→`query`; other tools match against the compact JSON of the
params. `<workspace>` expands to the canonical persona workspace plus
configured extra roots. File-path subjects are canonicalized (symlinks resolved;
non-existent targets resolve their deepest existing ancestor) *before*
matching, so `..` and symlink escapes are matched by their real location.

Config (`config.toml`):

```toml
[permissions]
deny  = ["bash(rm -rf *)"]
ask   = ["bash(git push*)"]
allow = ["bash(git status*)", "web-fetch(https://docs.rs/*)"]
workspace_roots = ["~/code/scratch"]   # extra roots besides the persona workspace

[permissions.decision_model]
enabled = true
model = "typesafe/jev-1.13"            # must be pinned; "*latest*" is rejected at load
threshold = 0.9
timeout_secs = 8
base_url = "https://openrouter.ai/api/alpha/decisions"
api_key_env = "OPENROUTER_API_KEY"
```

### D4. Prompt-injection posture of the decision model

The model sees: tool name, arguments, and the user's task text. It never sees
tool outputs, fetched content, or prior assistant text. Arguments are sent as
quoted data. Because the score is steerable, (a) every deterministic deny stays
upstream of it, (b) the threshold is high, and (c) the SSRF / confinement /
limit guards in the tools are enforced regardless of the verdict.

### D5. Tool-level guards (independent of the pipeline)

- **File tools** resolve the path through one core `PathGuard`
  (canonicalize + symlink resolution + root check) and refuse outside the
  roots. Roots: persona workspace + `permissions.workspace_roots`.
- **SSRF guard** in `tool-executor` and `workflow-http`: resolve host, reject
  loopback, private (10/8, 172.16/12, 192.168/16), link-local (169.254/16,
  fe80::/10 — covers cloud metadata), ULA (fc00::/7), unspecified, multicast,
  IPv4-mapped IPv6 of any of those; connect to the *resolved* address (no
  re-resolve → no DNS rebinding); disable automatic redirects and re-check each
  hop manually, capped at 5.
- **bash**: `timeout_secs` clamped to `permissions.limits.bash_max_timeout_secs`;
  stdout/stderr read incrementally and truncated at the cap while the child is
  still running; image auto-attach only for files inside workspace roots.
- **process**: sessions expire after `process_ttl_secs`, at most
  `process_max_sessions` live, per-line cap on stored output.

### D6. Observability

One `tracing` event per decision on target `assistant::permission`, fields
`tool`, `decider`, `rule`, `confidence`, `outcome`, `interface`,
`interactive`. The existing OTel tool span gets `permission.outcome` and
`permission.decider` attributes. Task context and arguments are not logged
(PII); only the rule text / model id is.

### D7. No compatibility

`AutoDenyConfirmation` stays (it is the non-interactive callback). The implicit
"non-interactive ⇒ skip confirmation" path is removed. Existing tests and docs
that rely on ungated execution are migrated, not shimmed.

## Risks / Trade-offs

- **Scheduled tasks break until configured.** Intended; the owner opted out of
  compatibility. Mitigated by built-in allow defaults for workspace-confined file
  tools and read-only tools, and documented `allow` examples.
- **Model latency** on every undecided call (8 s timeout). Static allow rules are
  the fast path; the model is the long tail.
- **Jev is steerable.** See D4; the model is an optimisation over asking a
  human, never the boundary.
- **MCP server path** is not behind the pipeline (tracked follow-up).

## Migration Plan

Stack of PRs (each green on its own): (a) core pipeline + static rules +
dispatch/subagent integration + non-interactive deny; (b) file-tool
confinement; (c) SSRF guard; (d) bash/process limits; (e) decision-model
decider. No data migration. Rollback = revert the PR; no persisted state.

## Open Questions

- Should `ask` rules in non-interactive turns route to the decision model
  instead of denying? Current answer: no — `ask` means "human only".
