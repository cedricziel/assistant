//! Workspace lint: `assistant-test-support` is dev-deps only.
//!
//! Asserts the contract introduced by the `workspace-test-coverage-floor`
//! OpenSpec change (specs/persistence-trait-symmetry): the
//! `assistant-test-support` crate MUST appear only in `[dev-dependencies]`,
//! never `[dependencies]`. Production binaries must not link the test
//! composition layer.

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

const TEST_SUPPORT_CRATE: &str = "assistant-test-support";

#[test]
fn test_support_not_in_normal_dependencies() {
    let root = workspace_root();
    let mut violations: Vec<String> = Vec::new();

    for member in workspace_members() {
        let manifest_path = root.join(&member).join("Cargo.toml");
        if !manifest_path.exists() {
            continue;
        }
        let toml = read_toml(&manifest_path);
        if let Some(deps) = toml.get("dependencies").and_then(|d| d.as_table())
            && deps.contains_key(TEST_SUPPORT_CRATE)
        {
            violations.push(format!(
                "{}: lists {TEST_SUPPORT_CRATE} in [dependencies]; \
                 it MUST be in [dev-dependencies] only",
                member,
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "assistant-test-support appears in normal dependencies:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn test_support_crate_is_registered() {
    // Sanity check: the test-support crate itself is a workspace member.
    let members = workspace_members();
    assert!(
        members.iter().any(|m| m == "crates/test-support"),
        "crates/test-support missing from [workspace.members]; \
         members = {members:?}"
    );
}
