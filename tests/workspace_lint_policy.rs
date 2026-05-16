//! Workspace-wide lint policy enforcement.
//!
//! Asserts the contract introduced by the `workspace-lint-policy` OpenSpec change:
//! the root `Cargo.toml` declares a `[workspace.lints]` baseline, every workspace
//! member inherits it via `[lints] workspace = true`, and the deny-level crates
//! enforce the panic-free contract at the crate level rather than via per-file
//! `cfg_attr` blocks.

use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_toml(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    toml::from_str::<Value>(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn workspace_members() -> Vec<String> {
    let root = read_toml(&workspace_root().join("Cargo.toml"));
    root.get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .expect("[workspace.members] missing")
        .iter()
        .map(|v| v.as_str().expect("member must be a string").to_string())
        .collect()
}

/// Lints baked into the workspace baseline. Promoting one of these from `warn`
/// to `deny` is a per-crate decision recorded in the crate's own `[lints.clippy]`
/// override.
const REQUIRED_WORKSPACE_LINTS: &[&str] = &[
    "unwrap_used",
    "expect_used",
    "panic",
    "unimplemented",
    "todo",
];

/// Crates that have completed the ratchet from `warn` to `deny` for the
/// panic-free lint set. Crates that still contain non-test panics elsewhere
/// in their source tree (e.g. `crates/web-ui`, which retains per-file
/// `#![cfg_attr(not(test), deny(...))]` on hot paths but is not fully clean
/// at the crate level) are not listed here and will be ratcheted in
/// follow-up changes.
const DENY_LEVEL_CRATES: &[&str] = &["crates/storage"];

#[test]
fn root_cargo_toml_declares_workspace_lint_table() {
    let root = read_toml(&workspace_root().join("Cargo.toml"));
    let clippy = root
        .get("workspace")
        .and_then(|w| w.get("lints"))
        .and_then(|l| l.get("clippy"))
        .expect("[workspace.lints.clippy] is missing from root Cargo.toml");

    for key in REQUIRED_WORKSPACE_LINTS {
        assert!(
            clippy.get(*key).is_some(),
            "[workspace.lints.clippy] is missing key `{key}`"
        );
    }

    let rust = root
        .get("workspace")
        .and_then(|w| w.get("lints"))
        .and_then(|l| l.get("rust"))
        .expect("[workspace.lints.rust] is missing from root Cargo.toml");
    assert!(
        rust.get("unused_must_use").is_some(),
        "[workspace.lints.rust.unused_must_use] is missing"
    );
}

#[test]
fn every_workspace_member_declares_a_lints_table() {
    // Cargo forbids combining `[lints] workspace = true` with per-crate
    // `[lints.clippy]` overrides in the same manifest. Each member must
    // either (a) inherit the workspace lints verbatim, or (b) declare its
    // own `[lints.clippy]` and `[lints.rust]` tables (the "manual replay"
    // pattern used by dirty crates with an active ratchet TODO).
    for member in workspace_members() {
        let manifest = workspace_root().join(&member).join("Cargo.toml");
        let toml = read_toml(&manifest);
        let lints = toml.get("lints").unwrap_or_else(|| {
            panic!(
                "{member}/Cargo.toml is missing the `[lints]` table; expected either \
                 `[lints]\\nworkspace = true` or explicit `[lints.clippy]`/`[lints.rust]` \
                 blocks with a ratchet TODO."
            )
        });

        let inherits = lints
            .get("workspace")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_clippy_override = lints.get("clippy").is_some();

        assert!(
            inherits || has_clippy_override,
            "{member}/Cargo.toml must contain either `[lints]\\nworkspace = true` \
             or an explicit `[lints.clippy]` override block."
        );
        assert!(
            !(inherits && has_clippy_override),
            "{member}/Cargo.toml cannot combine `[lints]\\nworkspace = true` with \
             a `[lints.clippy]` override — Cargo rejects this. Choose one."
        );
    }
}

#[test]
fn deny_level_crates_enforce_panic_free_contract() {
    for crate_dir in DENY_LEVEL_CRATES {
        let manifest = workspace_root().join(crate_dir).join("Cargo.toml");
        let toml = read_toml(&manifest);
        let clippy = toml
            .get("lints")
            .and_then(|l| l.get("clippy"))
            .unwrap_or_else(|| {
                panic!(
                    "{crate_dir}/Cargo.toml must declare `[lints.clippy]` with deny-level overrides"
                )
            });

        for lint in ["unwrap_used", "expect_used", "panic"] {
            let entry = clippy.get(lint).unwrap_or_else(|| {
                panic!("{crate_dir}/Cargo.toml [lints.clippy] missing `{lint}`")
            });
            let level = match entry {
                Value::String(s) => s.clone(),
                Value::Table(t) => t
                    .get("level")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .unwrap_or_default(),
                _ => String::new(),
            };
            assert_eq!(
                level, "deny",
                "{crate_dir}/Cargo.toml [lints.clippy].{lint} must be `deny`, got `{level}`"
            );
        }
    }
}

#[test]
fn deny_level_source_files_do_not_duplicate_panic_free_attribute() {
    // Only check source files for crates that enforce the contract at the
    // crate (Cargo.toml) level. Crates still using per-file `cfg_attr` blocks
    // (e.g. web-ui's hot paths) are not in scope until they ratchet to
    // crate-level deny.
    let suspects = ["crates/storage/src/lib.rs"];

    for suspect in suspects {
        let path = workspace_root().join(suspect);
        let body =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let normalized: String = body.split_whitespace().collect();
        assert!(
            !normalized.contains("deny(clippy::unwrap_used"),
            "{suspect} still declares an inline `deny(clippy::unwrap_used, ...)` block; \
             that contract now lives in the crate-level Cargo.toml [lints.clippy] override."
        );
    }
}
