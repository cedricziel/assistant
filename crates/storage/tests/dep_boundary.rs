//! Architectural guard: `assistant-storage` MUST depend only on `assistant-core`
//! and `assistant-skills` among workspace path dependencies. See
//! `openspec/changes/resolve-priority-tech-debt/specs/storage-layering/spec.md`.
//!
//! `assistant-skills` is an intentional carve-out (skills are a
//! storage-adjacent leaf domain).
//!
//! Parses `Cargo.toml` with the `toml` crate so the check is robust against
//! reformatting (multi-line inline tables, dotted-key form, etc.). Only the
//! `[dependencies]` table is inspected — `[dev-dependencies]` and
//! `[build-dependencies]` are explicitly excluded.

use std::fs;
use std::path::PathBuf;

const ALLOWED_PATH_DEPS: &[&str] = &["assistant-core", "assistant-skills"];

#[test]
fn storage_only_depends_on_core_and_skills() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let raw = fs::read_to_string(&manifest_path)
        .expect("reading crates/storage/Cargo.toml should succeed");

    let manifest: toml::Value =
        toml::from_str(&raw).expect("crates/storage/Cargo.toml should be valid TOML");

    let deps = manifest
        .get("dependencies")
        .and_then(|v| v.as_table())
        .expect("[dependencies] table missing from Cargo.toml");

    let mut violations: Vec<String> = Vec::new();
    for (name, value) in deps {
        if !is_path_dependency(value) {
            continue;
        }
        if !ALLOWED_PATH_DEPS.contains(&name.as_str()) {
            violations.push(name.clone());
        }
    }

    assert!(
        violations.is_empty(),
        "assistant-storage must only depend on workspace crates {:?}; \
         found disallowed path deps: {:?}",
        ALLOWED_PATH_DEPS,
        violations,
    );
}

/// True iff `value` is a path dependency — either a bare string (rare for
/// workspace crates) or an inline/regular table containing a `path` key.
fn is_path_dependency(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(t) => t.contains_key("path"),
        // A bare string version like `"1.0"` is never a path dep.
        _ => false,
    }
}
