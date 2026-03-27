<!--
SYNC IMPACT REPORT
==================
Version change: 1.0.0 → 1.1.0
Modified principles: none renamed
Added sections:
  - VIII. Dual-Mode Parity (NON-NEGOTIABLE) — new principle
Removed sections: none
Templates requiring updates:
  - .specify/templates/plan-template.md ✅ Constitution Check section present; no structural change needed
  - .specify/templates/spec-template.md ✅ No new mandatory sections required
  - .specify/templates/tasks-template.md ✅ Dual-mode testing note covered by principle; no template change needed
  - .specify/templates/commands/ ✅ No command files present
Deferred items:
  - RATIFICATION_DATE remains 2026-03-27 (first population date; original project start unknown)
-->

# assistant Constitution

## Core Principles

### I. Crate-First Modularity

Every feature MUST live in its own crate under `crates/` with a clear, single responsibility.
Crates MUST be independently compilable, testable, and documented.
No organizational-only crates are permitted — every crate MUST have a clear purpose.
Dependency direction MUST follow the declared order:
`interfaces → runtime → llm/tool-executor → storage → core`.
Circular dependencies between crates are NEVER permitted.

**Rationale**: Enforces separation of concerns, speeds up incremental compilation, and makes
each subsystem independently auditable and replaceable.

### II. Trait-Based Dependency Injection

All cross-crate dependencies MUST be injected as `Arc<dyn Trait>` — never as concrete types
across crate boundaries.
New shared abstractions MUST be declared as traits in `assistant-core` or the relevant boundary
crate before concrete implementations are created.
`Arc<T>` is acceptable for shared ownership of concrete types within a crate; `RwLock<HashMap<…>>`
for mutable registries.

**Rationale**: Enables mocking in unit tests, swapping implementations (e.g., LLM providers),
and prevents tight coupling between subsystems.

### III. Test Discipline

All async tests MUST use `#[tokio::test]`.
Unit tests MUST live in `#[cfg(test)] mod tests` at the bottom of the same file.
Tests that require a database MUST use `StorageLayer::new_in_memory()` — no disk I/O.
HTTP provider tests MUST use `wiremock`; no live network calls in unit or integration tests.
Tool handlers MUST return `Ok(ToolOutput::error(…))` for non-fatal errors visible to the LLM,
and `Err(…)` only for truly unrecoverable failures.

**Rationale**: Consistent test patterns keep CI deterministic. In-memory SQLite ensures full
migration coverage without disk state leakage between tests.

### IV. Observability

All library crates MUST use `tracing` macros (`debug!`, `info!`, `warn!`, `error!`).
`println!` is NEVER permitted in library crates.
The ReAct orchestration loop MUST emit OpenTelemetry spans for each turn.
Structured trace data MUST be persisted to SQLite via the `opentelemetry-exporter-sqlite` crate.
Module-level docs use `//!`; function docs `///`; section dividers `// -- Name --`.

**Rationale**: Passive trace logging enables the self-improvement feedback loop (skill refinement
proposals) and makes runtime behavior inspectable without modifying code.

### V. Simplicity and YAGNI

Abstractions MUST NOT be created for hypothetical future requirements.
Three similar lines of code are preferred over a premature helper.
Backwards-compatibility shims, unused `_var` renames, and re-export aliases for removed items
MUST NOT be added.
Complexity beyond the minimum needed for the current task MUST be explicitly justified in the
relevant ADR (`docs/adr/`).

**Rationale**: Premature abstractions increase cognitive overhead and make the codebase harder
to change. Minimalism keeps the codebase auditable.

### VI. Interface Parity via Orchestrator

Every new interface (CLI, Slack, Mattermost, Nextcloud, Signal, Web UI, or future) MUST route
all user turns through the shared `Orchestrator` in `assistant-runtime`.
Interface-specific business logic is NEVER permitted — interfaces MUST be thin adapters only.
Ambient capabilities exposed by an active interface MUST be registered into the shared skill
executor, not hard-coded into the interface.

**Rationale**: Guarantees consistent behaviour (system prompt, tools, skills, memory) across all
surfaces and prevents divergence between interfaces.

### VII. Code Quality Gate (NON-NEGOTIABLE)

The following checks MUST pass before any commit reaches `main`:

- `cargo fmt --all` (no diffs)
- `cargo clippy --workspace -- -D warnings` (zero warnings)
- `cargo machete --with-metadata` (no unused dependencies)

All commits MUST follow semantic-commit format with crate scope:
`feat(runtime): …`, `fix(storage): …`, `chore(cli): …`.
Prefixes: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`.
ADRs (`docs/adr/`) MUST be created or updated for every architectural decision.

**Rationale**: Enforced by pre-commit hooks (`make install-hooks`). Zero-warning policy keeps
the signal-to-noise ratio high in compiler output.

### VIII. Dual-Mode Parity (NON-NEGOTIABLE)

The system MUST remain fully functional in both run modes:

- **Single-binary mode** — all components run in one process via `assistant orchestrator run`.
  The in-process scheduler, message bus, and interfaces all start together.
- **Distributed mode** — components run as separate processes connected via an external message
  bus (e.g., NATS JetStream). Workers are started with `assistant worker`; the orchestrator
  runs with `--no-repl` or a subset of `--interfaces`.

Every user-facing capability MUST work correctly in both modes.
It is NEVER acceptable to ship a change that works in one mode but fails in the other.
New features MUST be designed and tested against both modes before merging.
CI MUST include at minimum a smoke path for each mode; a passing single-binary test does NOT
imply distributed-mode correctness.

**Rationale**: Users run the assistant both as a lightweight single-process install and as a
horizontally-scaled distributed system. Treating either mode as second-class will silently
break production deployments. The `MessageBus` abstraction (in-process vs. NATS) exists
precisely to keep both modes first-class — this principle enforces that intent.

## Code Style Standards

The following constraints apply workspace-wide:

- **Error handling**: `anyhow::Result` everywhere; `thiserror` only at explicit library
  boundary error types (rare). Use `anyhow::bail!` for early returns; `.with_context(|| "…")`
  to annotate failures.
- **Async**: `tokio` runtime with `features = ["full"]`; `#[async_trait]` on all traits with
  async methods; `tokio::fs` for file I/O in tool handlers.
- **Naming**: crates `assistant-*`, modules `snake_case`, structs/traits `PascalCase`, handler
  structs `<Feature>Handler`, tool names `kebab-case`, constants `SCREAMING_SNAKE`.
- **Terminology**: Use `Persona`, `Subagent Process`, and `A2A Profile` in all new docs and
  UX copy. Avoid unqualified `agent` in architecture prose. Canonical definitions live in
  `docs/glossary.md`.
- **Workspace deps**: All shared dependencies MUST be declared in `[workspace.dependencies]`
  in the root `Cargo.toml`; individual crates inherit with `dep.workspace = true`.
- **Frontmatter**: Use `gray_matter` for `SKILL.md` parsing; `serde_yaml` only in the A2A
  agent store. No ad-hoc YAML parsing elsewhere.

## Development Workflow

- **Pre-commit hooks**: Install with `make install-hooks` after cloning. Hooks enforce `fmt`,
  `clippy`, and `machete`. Never skip with `--no-verify` without explicit user approval.
- **Build targets**: `make build` (full), `make check` (fast), `make test` (unit),
  `make test-integration` (requires Ollama), `make lint`, `make format`.
- **Adding a builtin tool**: Follow the 5-step checklist in `AGENTS.md` — handler struct →
  `ToolHandler` impl → export from `mod.rs` → register in `ToolExecutor::register_builtins()`
  → optional `skills/<name>/SKILL.md`.
- **Dual-mode testing**: When adding or modifying runtime behaviour, verify the change works
  under both `assistant orchestrator run` (single-binary) and with `assistant worker` +
  external bus (distributed). Document which modes were tested in the PR description.
- **Architectural changes**: MUST be accompanied by a new or updated ADR in `docs/adr/`.
- **CI gates**: GitHub Actions runs check, test, lint, and format on every push to `main` and
  on every PR. The `signal` feature is linted separately. Integration tests run with
  `continue-on-error: true` (require Ollama).
- **Speckit workflow**: Use `/speckit.specify` → `/speckit.plan` → `/speckit.tasks` →
  `/speckit.implement` for non-trivial features. Specs live under `specs/<###-feature-name>/`.

## Governance

This constitution supersedes all other practices within the workspace. Where a conflict exists
between this document and any other guide, this document takes precedence.

**Amendment procedure**:

1. Open a PR with the proposed change to `.specify/memory/constitution.md`.
2. Update the Sync Impact Report comment at the top of this file.
3. Bump `CONSTITUTION_VERSION` following semver:
   - MAJOR — backward-incompatible principle removal or redefinition.
   - MINOR — new principle or material expansion of guidance.
   - PATCH — clarifications, wording, or typo fixes.
4. Propagate changes to dependent templates (plan, spec, tasks) in the same PR.
5. Add or update the relevant ADR in `docs/adr/`.

**Compliance**: All PRs MUST be verified against the principles before merge. Complexity
violations MUST be documented in `Complexity Tracking` sections of the feature plan.
Runtime development guidance lives in `AGENTS.md`; this constitution governs the _why_,
`AGENTS.md` governs the _how_.

**Version**: 1.1.0 | **Ratified**: 2026-03-27 | **Last Amended**: 2026-03-27
