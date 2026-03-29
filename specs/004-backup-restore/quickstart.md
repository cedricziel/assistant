# Developer Quickstart: Backup and Restore

**Feature**: 004-backup-restore
**Date**: 2026-03-29

## Prerequisites

- Rust toolchain (stable)
- `make install-hooks` run after clone
- `make build` passes

## Building

```sh
# Build just the new crate
cargo build -p assistant-backup

# Build the CLI (includes backup subcommands)
cargo build -p assistant-cli
```

## Running Tests

```sh
# All tests in the backup crate (no live install required)
cargo test -p assistant-backup

# With output
cargo test -p assistant-backup -- --nocapture

# Single test
cargo test -p assistant-backup test_backup_round_trip
```

## Manual Smoke Test

```sh
# Create a backup (uses ~/.assistant/ by default)
cargo run -p assistant-cli -- backup

# Specify output path
cargo run -p assistant-cli -- backup --output /tmp/my-backup.tar.gz

# List backups
cargo run -p assistant-cli -- backup list

# Restore (interactive confirmation)
cargo run -p assistant-cli -- restore /tmp/my-backup.tar.gz

# Restore without prompt (for scripts)
cargo run -p assistant-cli -- restore /tmp/my-backup.tar.gz --force

# Inspect archive contents manually
tar tf /tmp/my-backup.tar.gz
```

## New Crate Location

```
crates/backup/
├── Cargo.toml         # package name: assistant-backup
└── src/
    ├── lib.rs         # BackupEngine, RestoreEngine, public API
    ├── manifest.rs    # BackupManifest, ManifestEntry (serde)
    ├── archive.rs     # tar.gz creation/extraction helpers
    ├── checksum.rs    # SHA-256 helpers
    └── paths.rs       # Installation directory discovery
```

## Adding to Workspace

After creating `crates/backup/Cargo.toml`, add to root `Cargo.toml`:

```toml
[workspace]
members = [
  # ... existing members ...
  "crates/backup",
]

[workspace.dependencies]
# Add new deps:
tar = "0.4"
flate2 = { version = "1", default-features = false, features = ["zlib"] }
sha2 = "0.10"
hex = "0.4"
```

## Key Types

```rust
use assistant_backup::{BackupEngine, BackupOptions, RestoreEngine, RestoreOptions};

// Create a backup
let engine = BackupEngine::new();
let result = engine.run(BackupOptions {
    install_dir: PathBuf::from("/home/user/.assistant"),
    output_path: PathBuf::from("/tmp/backup.tar.gz"),
    db_path: None,
}).await?;

// Restore from backup
let engine = RestoreEngine::new();
let result = engine.run(RestoreOptions {
    archive_path: PathBuf::from("/tmp/backup.tar.gz"),
    install_dir: PathBuf::from("/home/user/.assistant"),
    force: true,
}).await?;
```
