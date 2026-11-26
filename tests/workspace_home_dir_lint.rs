//! Workspace lint: only the entry points look up the home directory.
//!
//! The install root (`~/.assistant`) is resolved once, by the binary that
//! starts the process, and handed down as a path. Library code that calls
//! `dirs::home_dir()` itself writes into the developer's real home whenever
//! a test reaches it. This lint bans `home_dir()` in every source file under
//! `crates/`, test code included, except the files listed below.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const BANNED_PATTERN: &str = "home_dir()";

/// Files that may call `home_dir()`, relative to the workspace root.
const ALLOWED_PATHS: &[&str] = &[
    // Entry points: each resolves the install root once and passes it down.
    "crates/interface-cli/src/main.rs",
    "crates/web-ui/src/lib.rs",
    // Expansion of `~` in paths the user wrote into config files.
    "crates/core/src/home.rs",
    // Not yet injected. No test writes through these; remove each entry
    // once its caller passes the install root in.
    "crates/backup/src/paths.rs",
    "crates/interface-cli/src/credentials.rs",
    "crates/llm-provider/src/openai/oauth.rs",
];

#[test]
fn home_dir_is_only_resolved_by_allowed_files() {
    let violations = scan(&workspace_root().join("crates"), &workspace_root());

    assert!(
        violations.is_empty(),
        "`home_dir()` outside the allowed files; take the install root as a \
         parameter instead:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn test_code_is_scanned_too() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("crates/demo/tests/it.rs");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "fn t() { let _ = dirs::home_dir(); }\n").unwrap();

    let violations = scan(&root.path().join("crates"), root.path());

    assert_eq!(
        violations.len(),
        1,
        "expected one violation: {violations:?}"
    );
}

#[test]
fn comments_are_not_flagged() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("crates/demo/src/lib.rs");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "/// Unlike `dirs::home_dir()`, this is injected.\n").unwrap();

    let violations = scan(&root.path().join("crates"), root.path());

    assert!(violations.is_empty(), "flagged a comment: {violations:?}");
}

fn scan(crates_dir: &Path, root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    collect_rust_files(crates_dir, &mut files);
    let mut violations = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        if ALLOWED_PATHS.contains(&rel.as_str()) {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("//") && line.contains(BANNED_PATTERN) {
                violations.push(format!("{rel}:{}", idx + 1));
            }
        }
    }
    violations
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            if !matches!(
                path.file_name().and_then(|n| n.to_str()),
                Some("target" | "node_modules")
            ) {
                collect_rust_files(&path, files);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}
