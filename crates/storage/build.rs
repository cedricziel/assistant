//! Build script for `assistant-storage`.
//!
//! Discovers the space.db migrations so adding one is just adding a file:
//! every top-level `migrations/*.sql` at the workspace root is registered,
//! sorted by file name, under its file stem (the name recorded in the
//! `_migrations` table). The result is written to
//! `$OUT_DIR/space_migrations.rs` as a `&[(&str, &str)]` of
//! `(name, include_str!(<absolute path>))` and included by `src/lib.rs`.
//!
//! `migrations/org/` is a separate set and is intentionally not scanned.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR is not set")?,
    );
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").context("OUT_DIR is not set")?);
    let migrations_dir = manifest_dir.join("..").join("..").join("migrations");

    println!("cargo:rerun-if-changed=build.rs");
    // Re-run when files are added or removed ...
    println!("cargo:rerun-if-changed={}", migrations_dir.display());

    let mut files = sql_files(&migrations_dir)?;
    files.sort();

    let mut generated = String::from("&[\n");
    for path in &files {
        // ... or edited (include_str! content).
        println!("cargo:rerun-if-changed={}", path.display());

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .with_context(|| format!("migration file name is not UTF-8: {}", path.display()))?;
        writeln!(
            generated,
            "    ({stem:?}, include_str!({:?})),",
            path.display().to_string()
        )?;
    }
    generated.push_str("]\n");

    std::fs::write(out_dir.join("space_migrations.rs"), generated)
        .context("failed to write space_migrations.rs")?;
    Ok(())
}

/// Canonicalised paths of the `.sql` files directly inside `dir`.
fn sql_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read migrations dir {}", dir.display()))?;
    for entry in entries {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "sql") {
            files.push(
                path.canonicalize()
                    .with_context(|| format!("failed to canonicalize {}", path.display()))?,
            );
        }
    }
    Ok(files)
}
