//! Workspace lint: test-only types must not appear in production code.
//!
//! Asserts the contract introduced by the `workspace-test-coverage-floor`
//! OpenSpec change (specs/persistence-trait-symmetry):
//!
//! Types whose name starts with `InMemory`, `Scripted`, or `Stub` are
//! considered test-only stand-ins. They MUST NOT be constructed in
//! non-test production code, with the following exemptions:
//!
//! - The file that defines the type (where `impl Default` / `impl <Type>`
//!   blocks construct it as part of the impl surface).
//! - Files in `crates/test-support/` — the test composition layer.
//! - Files under any `tests/` directory.
//! - Lines inside `#[cfg(test)] mod tests { ... }` blocks (heuristic:
//!   any line whose number is greater than the first `#[cfg(test)]`
//!   attribute in the file).
//! - The documented `EXEMPT_TYPE_NAMES` list below (production fallbacks
//!   that happen to share the `InMemory` naming convention because they
//!   are legitimately non-persistent).
//!
//! As more in-memory impls land in later phases (Phase 5 persistence,
//! Phase 7 orchestration stubs), contributors keep this lint passing by
//! keeping their construction inside test code or by adding documented
//! production fallbacks to `EXEMPT_TYPE_NAMES`.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Types whose construction is allowed in production code despite the
/// `InMemory*` / `Scripted*` / `Stub*` prefix. Each entry is a documented
/// production fallback or compatibility shim.
const EXEMPT_TYPE_NAMES: &[&str] = &[
    // Used as the default `ConversationBroadcast` impl when no external
    // broadcaster (NATS, web-ui SSE multiplexer) is configured. Defined in
    // `crates/storage/src/conversation_broadcaster.rs`; constructed in
    // `crates/web-ui/src/api/mod.rs::ApiState::new` and analogous locations.
    "InMemoryConversationBroadcaster",
    // Private inner-state structs of the public `StubOrchestrationEngine`
    // and `ScriptedLlmProvider` types. Constructed only inside their
    // wrapping types `new()` methods.
    "StubData",
    "ScriptedData",
];

/// Crate directories under `crates/` exempt from the scan.
const EXEMPT_CRATES: &[&str] = &[
    // The test composition layer is allowed to construct any fake.
    "test-support",
];

#[test]
fn no_test_only_impls_in_production_code() {
    let root = workspace_root().join("crates");
    let mut violations: Vec<String> = Vec::new();

    for entry in fs::read_dir(&root).expect("read crates/") {
        let entry = entry.expect("dir entry");
        if !entry.path().is_dir() {
            continue;
        }
        let crate_dir_name = entry.file_name().to_string_lossy().into_owned();
        if EXEMPT_CRATES.contains(&crate_dir_name.as_str()) {
            continue;
        }
        let src_dir = entry.path().join("src");
        if !src_dir.is_dir() {
            continue;
        }
        scan_dir(&src_dir, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "test-only impls constructed in production code:\n  {}",
        violations.join("\n  ")
    );
}

fn scan_dir(dir: &Path, violations: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip in-tree `tests/` directories (some crates colocate them).
            if path.file_name().and_then(|n| n.to_str()) == Some("tests") {
                continue;
            }
            scan_dir(&path, violations);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        scan_file(&path, violations);
    }
}

fn scan_file(path: &Path, violations: &mut Vec<String>) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };

    // Compute the line number of the first `#[cfg(test)]` attribute, if any.
    // Lines at or after this number are treated as test code by the
    // heuristic. This is an approximation: it works for the common pattern
    // of `#[cfg(test)] mod tests { ... }` at the bottom of a file, but
    // would over-exempt files that gate a single `use` statement with
    // `#[cfg(test)]` near the top. Refine if false negatives appear.
    let first_cfg_test_line: Option<usize> = content.lines().enumerate().find_map(|(idx, line)| {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            Some(idx + 1)
        } else {
            None
        }
    });

    // The file is permitted to construct types it defines.
    let defines_type = |type_name: &str| -> bool {
        let needle = format!("pub struct {type_name}");
        content.contains(&needle)
    };

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(cfg_test_line) = first_cfg_test_line
            && line_no >= cfg_test_line
        {
            continue;
        }

        for type_name in find_test_only_constructions(line) {
            if EXEMPT_TYPE_NAMES.contains(&type_name.as_str()) {
                continue;
            }
            if defines_type(&type_name) {
                continue;
            }
            violations.push(format!(
                "{}:{} constructs test-only type `{type_name}`",
                path.display(),
                line_no,
            ));
        }
    }
}

/// Find type names of the form `(InMemory|Scripted|Stub)<PascalCase>`
/// followed by `::` on the given line.
fn find_test_only_constructions(line: &str) -> Vec<String> {
    let mut hits: Vec<String> = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        for prefix in ["InMemory", "Scripted", "Stub"] {
            let p = prefix.as_bytes();
            if i + p.len() < bytes.len() && bytes[i..i + p.len()].eq_ignore_ascii_case(p) {
                // Must be at a word boundary on the left.
                if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
                    continue;
                }
                let start = i;
                let mut end = i + p.len();
                // Continue across PascalCase identifier characters.
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                // The match must be followed by `::` to count as a
                // constructor / associated-item reference. This avoids
                // false positives on type-name mentions in comments,
                // docstrings, or `dyn TraitName` references.
                if end + 1 < bytes.len() && &bytes[end..end + 2] == b"::" {
                    // Require at least one extra character after the
                    // prefix so we don't match the bare prefix word.
                    if end > i + p.len() {
                        let name = String::from_utf8_lossy(&bytes[start..end]).to_string();
                        hits.push(name);
                    }
                }
                i = end;
                break;
            }
        }
        i += 1;
    }
    hits
}
