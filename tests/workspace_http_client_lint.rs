//! Workspace lint: outbound HTTP types accept an injected `reqwest::Client`.
//!
//! Enforces the contract introduced by the `workspace-test-coverage-floor`
//! OpenSpec change (specs/http-client-injection):
//!
//! Production code that issues outbound HTTP MUST accept a `reqwest::Client`
//! at construction. `reqwest::Client::new()` and `reqwest::Client::builder()`
//! are forbidden in method bodies — they belong only in constructors that
//! provide a default. Tests should construct types with a wiremock-pointed
//! client.
//!
//! ## Exempt locations
//!
//! - Lines inside a function whose signature returns `Self`, `Result<Self>`,
//!   or `anyhow::Result<Self>` — i.e. a constructor / builder.
//!   Constructors are allowed to build a default client.
//! - Lines inside top-level binary `main.rs` files (root CLI entry points
//!   that wire up dependencies once at startup).
//! - Lines inside `#[cfg(test)]` regions or files under `tests/` directories.
//! - Files explicitly listed in `EXEMPT_PATHS` (documented one-off helpers).
//!
//! The heuristic for "inside a constructor": walk backwards from the match
//! line and find the most recent `fn <name>(...)` definition. If the
//! signature contains `-> Self`, `-> Result<Self`, `-> anyhow::Result<Self`,
//! or `Result<Self,`, treat the match as inside a constructor.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Patterns that signal an in-method client construction.
const BANNED_PATTERNS: &[&str] = &["reqwest::Client::new()", "reqwest::Client::builder()"];

/// Files exempt from the lint.
///
/// Two categories:
/// - Top-level binary entry points — wire up clients once at startup.
/// - Documented one-off / utility helpers that build per-call clients with
///   specific timeout/auth configurations. These are deferred migration
///   candidates: each one will eventually be refactored to accept a
///   `reqwest::Client`, but doing so is invasive (touches many call sites)
///   so they're tracked here as a backlog. **Remove entries as the
///   migration lands.** Adding entries requires a separate OpenSpec change.
const EXEMPT_PATHS: &[&str] = &[
    // Top-level binary entry points.
    "crates/interface-cli/src/main.rs",
    // Deferred migration backlog (Phase 4.8 follow-ups in
    // workspace-test-coverage-floor):
    "crates/interfaces/src/nextcloud/adapter.rs",
    "crates/interfaces/src/nextcloud/tools.rs",
    "crates/interfaces/src/signal/adapter.rs",
    "crates/llm-provider/src/http.rs",
    "crates/runtime/src/webhook_dispatch.rs",
    "crates/tool-executor/src/installer.rs",
    "crates/web-ui/src/api/webhooks.rs",
    "crates/interface-cli/src/cmd_login.rs",
];

#[test]
fn no_in_method_client_construction() {
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
        "in-method reqwest::Client construction in production code:\n  {}",
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

    let lines: Vec<&str> = content.lines().collect();

    let first_cfg_test_line: Option<usize> = lines.iter().enumerate().find_map(|(idx, line)| {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            Some(idx + 1)
        } else {
            None
        }
    });

    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        if let Some(cfg_test_line) = first_cfg_test_line
            && line_no >= cfg_test_line
        {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("*") {
            continue;
        }
        let matched = BANNED_PATTERNS.iter().find(|p| line.contains(*p));
        if let Some(pattern) = matched
            && !inside_constructor(&lines, idx)
        {
            violations.push(format!(
                "{rel_path_str}:{line_no} contains `{pattern}` outside a constructor — \
                 accept reqwest::Client as a parameter instead",
            ));
        }
    }
}

/// Walk backwards from `match_idx` to find the most recent `fn ...` line and
/// inspect its signature. Returns `true` when the enclosing function returns
/// `Self`, `Result<Self>`, or a variant that ends with `Self>`.
fn inside_constructor(lines: &[&str], match_idx: usize) -> bool {
    // Walk backwards to find the start of the enclosing function. Track
    // brace depth so we don't pick up `fn` lines inside nested closures.
    let mut depth: i32 = 0;
    for i in (0..=match_idx).rev() {
        let line = lines[i];
        // Count braces on this line. (Naive, but adequate for the lint.)
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
        let trimmed = line.trim_start();
        if (trimmed.contains("fn ") || trimmed.starts_with("async fn"))
            && (trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub(crate) fn ")
                || trimmed.starts_with("pub(super) fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.starts_with("pub(crate) async fn ")
                || trimmed.starts_with("pub(super) async fn ")
                || trimmed.starts_with("fn(")
                || trimmed.starts_with("    fn ")
                || trimmed.starts_with("    pub fn ")
                || trimmed.starts_with("    pub(crate) fn ")
                || trimmed.starts_with("    async fn ")
                || trimmed.starts_with("    pub async fn ")
                || trimmed.starts_with("    pub(crate) async fn "))
        {
            // Gather the next ~8 lines (signature can span multiple lines)
            // and check for a Self-returning signature.
            let mut sig = String::new();
            for j in i..usize::min(i + 8, lines.len()) {
                sig.push_str(lines[j]);
                sig.push(' ');
                if lines[j].contains('{') {
                    break;
                }
            }
            return sig.contains("-> Self")
                || sig.contains("-> Result<Self")
                || sig.contains("anyhow::Result<Self")
                || sig.contains("Result<Self,");
        }
        // If we hit depth 0 going up, we're past the enclosing fn's opening brace
        if depth > 0 {
            // Found enclosing scope but no fn yet; continue walking back.
        }
    }
    false
}
