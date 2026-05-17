//! Workspace lint: production code must not call `Utc::now()` or
//! `SystemTime::now()` directly.
//!
//! Enforces the contract introduced by the `workspace-test-coverage-floor`
//! OpenSpec change (specs/clock-abstraction):
//!
//! All current-time reads in non-test production code go through an
//! injected `Arc<dyn Clock>` (preferred) or the canonical
//! `assistant_core::clock::SystemClock` wrapper. Direct calls to
//! `chrono::Utc::now()`, `chrono::Local::now()`, or `std::time::SystemTime::now()`
//! are banned outside the exempt paths listed below.
//!
//! ## Exempt paths
//!
//! - `crates/core/src/clock.rs` — the canonical `SystemClock` wrapper.
//!   `SystemClock::now()` IS the call site that wraps `Utc::now()`.
//! - `crates/storage/src/migration.rs` — bootstrap migration code with
//!   several callers; threading a clock would cascade through every
//!   migration call site. Documented exempt; a future refactor may
//!   tighten this.
//! - Any file under a `tests/` directory (sibling integration tests).
//! - Files that contain `#[cfg(test)]` and the match line falls inside
//!   the test region (heuristic: match line number >= first `#[cfg(test)]`
//!   line in the file). Same pattern as `workspace_test_impls_in_prod.rs`.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Banned patterns. Production code calls must go through `Arc<dyn Clock>`
/// or `SystemClock` (re-exported from `assistant_core::clock`).
const BANNED_PATTERNS: &[&str] = &["Utc::now()", "Local::now()", "SystemTime::now()"];

/// Paths exempt from this lint, relative to the workspace root.
const EXEMPT_PATHS: &[&str] = &[
    // The canonical SystemClock wrapper itself. SystemClock::now() must
    // call Utc::now() — it IS the wrapper.
    "crates/core/src/clock.rs",
    // Bootstrap migration code with cascading callers; documented exempt.
    "crates/storage/src/migration.rs",
];

#[test]
fn no_direct_clock_calls_in_production_code() {
    let root = workspace_root().join("crates");
    let mut violations: Vec<String> = Vec::new();

    for entry in fs::read_dir(&root).expect("read crates/") {
        let entry = entry.expect("dir entry");
        if !entry.path().is_dir() {
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
        "direct clock calls in production code:\n  {}",
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
            // Skip in-tree `tests/` directories (sibling integration tests).
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
    let root = workspace_root();
    let rel_path = path.strip_prefix(&root).unwrap_or(path);
    let rel_path_str = rel_path.to_string_lossy().to_string();

    if EXEMPT_PATHS.iter().any(|p| rel_path_str == *p) {
        return;
    }

    let Ok(content) = fs::read_to_string(path) else {
        return;
    };

    // First `#[cfg(test)]` attribute marks the start of the test region.
    let first_cfg_test_line: Option<usize> = content.lines().enumerate().find_map(|(idx, line)| {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            Some(idx + 1)
        } else {
            None
        }
    });

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(cfg_test_line) = first_cfg_test_line
            && line_no >= cfg_test_line
        {
            continue;
        }
        // Skip comments and doc lines.
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("*") {
            continue;
        }
        for pattern in BANNED_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!(
                    "{rel_path_str}:{line_no} contains banned `{pattern}` — use Arc<dyn Clock> or SystemClock instead",
                ));
            }
        }
    }
}
