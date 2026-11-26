//! Helpers shared by the workspace lint scanners.

use std::fs;
use std::path::{Path, PathBuf};

/// Line number of the first `#[cfg(test)]` attribute. The scanners treat
/// every line from there on as test code, which over-exempts a file that
/// gates an early item, such as a `use`, with `#[cfg(test)]`.
pub fn first_cfg_test_line(content: &str) -> Option<usize> {
    content
        .lines()
        .position(|line| line.trim_start().starts_with("#[cfg(test)]"))
        .map(|idx| idx + 1)
}

/// A source file read from a crate's `src/` tree.
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
}

/// The production source files of every crate below `crates_dir`, except
/// the crate directories named in `skip_crates`. Leaves out `tests/`
/// directories and files a parent module declares with
/// `#[cfg(test)] mod name;`, along with their submodules.
pub fn production_sources(crates_dir: &Path, skip_crates: &[&str]) -> Vec<SourceFile> {
    let mut production = Vec::new();
    for entry in fs::read_dir(crates_dir).expect("read crates/").flatten() {
        if skip_crates.contains(&entry.file_name().to_string_lossy().as_ref()) {
            continue;
        }
        let mut sources = Vec::new();
        read_sources(&entry.path().join("src"), &mut sources);
        let test_modules: Vec<PathBuf> = sources
            .iter()
            .flat_map(|source| {
                let children_dir = children_dir(&source.path);
                cfg_test_mod_declarations(&source.content)
                    .into_iter()
                    .map(move |name| children_dir.join(name))
            })
            .collect();
        production.extend(sources.into_iter().filter(|source| {
            let module = source.path.with_extension("");
            !test_modules.iter().any(|test| module.starts_with(test))
        }));
    }
    production
}

fn read_sources(dir: &Path, sources: &mut Vec<SourceFile>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) != Some("tests") {
                read_sources(&path, sources);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && let Ok(content) = fs::read_to_string(&path)
        {
            sources.push(SourceFile { path, content });
        }
    }
}

/// Directory holding the out-of-line submodules of `file`: the parent for
/// `mod.rs`, `lib.rs` and `main.rs`, otherwise `foo/` next to `foo.rs`.
fn children_dir(file: &Path) -> PathBuf {
    match file.file_name().and_then(|n| n.to_str()) {
        Some("mod.rs" | "lib.rs" | "main.rs") => file.parent().unwrap_or(file).to_path_buf(),
        _ => file.with_extension(""),
    }
}

/// Names of `mod name;` declarations preceded by `#[cfg(test)]`, either on
/// the same line or with only attributes and blank lines in between.
fn cfg_test_mod_declarations(content: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut under_cfg_test = false;
    for line in content.lines() {
        let mut line = line.trim();
        if let Some(rest) = line.strip_prefix("#[cfg(test)]") {
            under_cfg_test = true;
            line = rest.trim_start();
        }
        if line.is_empty() || line.starts_with("#[") {
            continue;
        }
        if under_cfg_test && let Some(name) = mod_declaration(line) {
            names.push(name);
        }
        under_cfg_test = false;
    }
    names
}

/// `name` from `mod name;`, `pub mod name;` or `pub(crate) mod name;`.
fn mod_declaration(line: &str) -> Option<&str> {
    let (visibility, rest) = line.split_once("mod ")?;
    if !(visibility.is_empty() || visibility.starts_with("pub")) {
        return None;
    }
    let name = rest.strip_suffix(';')?.trim();
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
        .then_some(name)
}
