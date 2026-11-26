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
//! - Files a parent module declares with `#[cfg(test)] mod name;`.

use std::fs;
use std::path::{Path, PathBuf};

use common::{first_cfg_test_line, production_sources};

mod common;

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
    let violations = scan_crates(&workspace_root().join("crates"));

    assert!(
        violations.is_empty(),
        "direct clock calls in production code:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn out_of_line_test_module_is_test_code() {
    let fixture = Fixture::new(&[
        ("demo/src/lib.rs", "mod foo;\n"),
        ("demo/src/foo/mod.rs", "#[cfg(test)]\nmod tests;\n"),
        ("demo/src/foo/tests.rs", "fn t() { let _ = Utc::now(); }\n"),
    ]);

    let violations = scan_crates(&fixture.crates_dir());

    assert!(violations.is_empty(), "flagged test code: {violations:?}");
}

#[test]
fn out_of_line_module_without_cfg_test_is_production_code() {
    let fixture = Fixture::new(&[
        ("demo/src/lib.rs", "mod foo;\n"),
        ("demo/src/foo/mod.rs", "mod tests;\n"),
        ("demo/src/foo/tests.rs", "fn t() { let _ = Utc::now(); }\n"),
    ]);

    let violations = scan_crates(&fixture.crates_dir());

    assert_eq!(
        violations.len(),
        1,
        "expected one violation: {violations:?}"
    );
}

/// A throwaway `crates/` tree, removed on drop.
pub struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    /// Writes each `(path, content)` pair below a fresh `crates/` directory.
    pub fn new(files: &[(&str, &str)]) -> Self {
        let root = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let path = root.path().join("crates").join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        Self { root }
    }

    pub fn crates_dir(&self) -> PathBuf {
        self.root.path().join("crates")
    }
}

fn scan_crates(crates_dir: &Path) -> Vec<String> {
    let mut violations = Vec::new();
    for source in production_sources(crates_dir, &[]) {
        scan_file(&source.path, &source.content, &mut violations);
    }
    violations
}

fn scan_file(path: &Path, content: &str, violations: &mut Vec<String>) {
    let root = workspace_root();
    let rel_path = path.strip_prefix(&root).unwrap_or(path);
    let rel_path_str = rel_path.to_string_lossy().to_string();

    if EXEMPT_PATHS.iter().any(|p| rel_path_str == *p) {
        return;
    }

    let first_cfg_test_line = first_cfg_test_line(content);

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
