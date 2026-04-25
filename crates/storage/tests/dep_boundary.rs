//! Architectural guard: `assistant-storage` MUST depend only on `assistant-core`
//! among workspace path dependencies. See
//! `openspec/changes/resolve-priority-tech-debt/specs/storage-layering/spec.md`.
//!
//! `assistant-skills` is an intentional carve-out for now (skills are a
//! storage-adjacent leaf domain; see the proposal's open questions).
//!
//! This test parses `Cargo.toml` directly and fails if any disallowed
//! workspace path dep reappears.

use std::fs;
use std::path::PathBuf;

const ALLOWED_PATH_DEPS: &[&str] = &["assistant-core", "assistant-skills"];

#[test]
fn storage_only_depends_on_core_and_skills() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let contents =
        fs::read_to_string(&manifest).expect("reading crates/storage/Cargo.toml should succeed");

    let mut violations: Vec<String> = Vec::new();
    let mut in_dependencies = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        // Section tracking: enter only the [dependencies] table, ignore
        // [dev-dependencies] and any other tables.
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if !in_dependencies {
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for `name = { path = "../<crate>" ...}` style entries.
        let Some((name, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if !rest.contains("path =") && !rest.contains("path=") {
            continue;
        }

        if !ALLOWED_PATH_DEPS.contains(&name) {
            violations.push(name.to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "assistant-storage must only depend on workspace crates {:?}; found disallowed path deps: {:?}",
        ALLOWED_PATH_DEPS,
        violations
    );
}
