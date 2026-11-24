# Tasks — tool-permission-decision-model

TDD throughout: failing test first, confirm RED, then GREEN. One PR per section.

## 1. Core pipeline, static rules, dispatch integration (PR a)

- [x] 1.1 RED/GREEN: `assistant_core::permission` types (`PermissionRequest`,
      `Verdict`, `Decision`, `Outcome`, `ToolDecider`).
- [x] 1.2 RED/GREEN: rule parser (`tool(pattern)`) and wildcard matcher, primary
      argument extraction per tool.
- [x] 1.3 RED/GREEN: `StaticRules` evaluation order (deny > ask > allow),
      built-in hard-deny lists (dangerous shell commands, credential paths); no network-destination deny,
      built-in safe allow defaults.
- [x] 1.4 RED/GREEN: `PermissionPolicy` pipeline with a scripted decider.
- [x] 1.5 `PermissionsConfig` in `AssistantConfig` (`[permissions]`).
- [x] 1.6 RED/GREEN: wire into `dispatch_global_tool` and the subagent loop;
      non-interactive `Ask` → `ToolOutput::error`-style denial returned to the
      LLM; decision tracing event; denial marks the OTel tool span.
- [x] 1.7 Docs (`docs/tool-permissions.md`) and ADR.

## 2. File-tool workspace confinement (PR b)

- [x] 2.1 `PathGuard` (canonicalize, symlink resolution, root check) in core.
- [x] 2.2 RED/GREEN: `file-read`, `file-write`, `file-edit`, `file-glob` refuse
      paths outside roots (`..`, absolute, `~`, symlink escape).
- [x] 2.3 `<workspace>` expansion in rules uses the same roots.

## 3. bash / process limits (PR c)

- [x] 3.1 RED/GREEN: `timeout_secs` clamped to config max.
- [x] 3.2 RED/GREEN: streaming output cap (child not fully buffered).
- [x] 3.3 RED/GREEN: image auto-attach restricted to workspace roots.
- [x] 3.4 RED/GREEN: `process` TTL, session cap, per-line cap.

## 4. Jev decision-model decider (PR d)

- [ ] 4.1 RED/GREEN: `DecisionModelDecider` against a wiremock server —
      approve ≥ threshold, below threshold, HTTP error, timeout, malformed body.
- [ ] 4.2 Config validation: reject unpinned / `latest` model ids.
- [ ] 4.3 Wire from `[permissions.decision_model]` into the orchestrator
      builder; docs + ADR update.
- [ ] 4.4 RED/GREEN: destination context (host, `private` flag, best-effort
      resolution) in the decision request for `web-fetch` / `workflow-http`;
      `web-fetch` reports the final URL after redirects.
