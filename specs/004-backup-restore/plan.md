# Implementation Plan: Backup and Restore Subcommands

**Branch**: `004-backup-restore` | **Date**: 2026-03-29 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/004-backup-restore/spec.md`

## Summary

Add `assistant backup` and `assistant restore` subcommands that create and restore `.tar.gz` archives of the full installation (`~/.assistant/`). The core backup/restore logic lives in a new `assistant-backup` crate (Constitution I), injected via a `BackupFs` trait to enable unit testing without disk I/O (Constitution II & III). The CLI crate remains a thin adapter.

## Technical Context

**Language/Version**: Rust 2021 edition, workspace resolver 2
**Primary Dependencies**: `tar 0.4`, `flate2 1`, `sha2 0.10`, `hex 0.4` (new); `serde`, `serde_json`, `tokio`, `anyhow`, `tracing`, `chrono` (existing workspace deps)
**Storage**: SQLite at `~/.assistant/assistant.db` (WAL-checkpointed before copy); no new tables
**Testing**: `cargo test` with `#[tokio::test]`; unit tests use in-memory `FakeFs`; integration tests use `tempfile::TempDir`
**Target Platform**: Linux, macOS (same as existing CLI)
**Project Type**: CLI tool + library crate
**Performance Goals**: Backup in ≤30 s for ≤50 MB DB + ≤100 personas; restore in ≤60 s (spec SC-001/SC-002)
**Constraints**: No live installation required for tests (SC-004); no partial artifacts on failure (SC-006)
**Scale/Scope**: Single-machine local filesystem; no cloud/network destinations in v1

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

| Principle                             | Status  | Notes                                                                                                           |
| ------------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------- |
| I. Crate-First Modularity             | ✅ Pass | New `assistant-backup` crate; CLI is a thin adapter                                                             |
| II. Trait-Based DI                    | ✅ Pass | `BackupFs` trait injected into `BackupEngine`/`RestoreEngine`                                                   |
| III. Test Discipline                  | ✅ Pass | `FakeFs` for unit tests; `tempfile` for integration tests; `#[tokio::test]`                                     |
| IV. Observability                     | ✅ Pass | `tracing` macros used throughout; no `println!` in library                                                      |
| V. YAGNI                              | ✅ Pass | No cloud upload, incremental backups, or per-persona scope in v1                                                |
| VI. Interface Parity via Orchestrator | ✅ N/A  | Backup is a maintenance operation; not a user turn; Orchestrator not involved                                   |
| VII. Code Quality Gate                | ✅ Pass | `fmt`, `clippy -D warnings`, `machete` enforced by pre-commit hooks                                             |
| VIII. Dual-Mode Parity                | ✅ N/A  | Backup/restore is a CLI maintenance command run when the assistant is idle; not part of runtime message routing |

No violations. Complexity Tracking table not required.

## Project Structure

### Documentation (this feature)

```text
specs/004-backup-restore/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── cli-commands.md  # CLI command contracts + manifest JSON schema
└── tasks.md             # Phase 2 output (/speckit.tasks — NOT yet created)
```

### Source Code

```text
crates/backup/                         # NEW: assistant-backup
├── Cargo.toml
└── src/
    ├── lib.rs                         # Public API: BackupEngine, RestoreEngine, BackupFs trait
    ├── manifest.rs                    # BackupManifest, ManifestEntry (serde)
    ├── archive.rs                     # tar.gz creation/extraction helpers
    ├── checksum.rs                    # SHA-256 helpers
    └── paths.rs                       # Installation path discovery (~/.assistant/)

crates/interface-cli/src/
└── cmd_backup.rs                      # NEW: clap subcommand definitions + thin adapter calls

# Modified files:
Cargo.toml                             # Add workspace members + new deps (tar, flate2, sha2, hex)
crates/interface-cli/src/main.rs       # Wire Backup/Restore/BackupList subcommands
crates/interface-cli/Cargo.toml        # Add assistant-backup dep
```

**Structure Decision**: Single new library crate (`assistant-backup`) depended on by `assistant-cli`. Follows the established pattern of `assistant-interface-*` and `assistant-provider-*` crates. No new interface crate needed; backup is not an interactive interface.

## Complexity Tracking

_(No constitution violations — table not required)_
