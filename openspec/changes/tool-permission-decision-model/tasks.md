# Tasks — tool-permission-decision-model

TDD throughout: failing test first, confirm RED, then GREEN. One PR per section.

## 1. Core pipeline, static rules, dispatch integration (PR a)

- [ ] 1.1 RED/GREEN: `assistant_core::permission` types (`PermissionRequest`,
      `Verdict`, `Decision`, `Outcome`, `ToolDecider`).
- [ ] 1.2 RED/GREEN: rule parser (`tool(pattern)`) and wildcard matcher, primary
      argument extraction per tool.
- [ ] 1.3 RED/GREEN: `StaticRules` evaluation order (deny > ask > allow),
      built-in hard-deny lists (dangerous shell commands, SSRF target literals),
      built-in safe allow defaults.
- [ ] 1.4 RED/GREEN: `PermissionPolicy` pipeline with a scripted decider.
- [ ] 1.5 `PermissionsConfig` in `AssistantConfig` (`[permissions]`).
- [ ] 1.6 RED/GREEN: wire into `dispatch_global_tool` and the subagent loop;
      non-interactive `Ask` → `ToolOutput::error`-style denial returned to the
      LLM; decision tracing event + OTel span attributes.
- [ ] 1.7 Docs (`docs/tool-permissions.md`) and ADR.

## 2. File-tool workspace confinement (PR b)

- [ ] 2.1 `PathGuard` (canonicalize, symlink resolution, root check) in core.
- [ ] 2.2 RED/GREEN: `file-read`, `file-write`, `file-edit`, `file-glob` refuse
      paths outside roots (`..`, absolute, `~`, symlink escape).
- [ ] 2.3 `<workspace>` expansion in rules uses the same roots.

## 3. SSRF guard (PR c)

- [ ] 3.1 RED/GREEN: address classifier (loopback/private/link-local/ULA/
      metadata/mapped).
- [ ] 3.2 RED/GREEN: `web-fetch` resolves, pins the address, re-checks every
      redirect hop (cap 5).
- [ ] 3.3 RED/GREEN: same for `crates/workflow-http`.

## 4. bash / process limits (PR d)

- [ ] 4.1 RED/GREEN: `timeout_secs` clamped to config max.
- [ ] 4.2 RED/GREEN: streaming output cap (child not fully buffered).
- [ ] 4.3 RED/GREEN: image auto-attach restricted to workspace roots.
- [ ] 4.4 RED/GREEN: `process` TTL, session cap, per-line cap.

## 5. Jev decision-model decider (PR e)

- [ ] 5.1 RED/GREEN: `DecisionModelDecider` against a wiremock server —
      approve ≥ threshold, below threshold, HTTP error, timeout, malformed body.
- [ ] 5.2 Config validation: reject unpinned / `latest` model ids.
- [ ] 5.3 Wire from `[permissions.decision_model]` into the orchestrator
      builder; docs + ADR update.
