//! `assistant reset` — wipe DB + core memory files and re-seed defaults.

use std::io::{self, Write as IoWrite};
use std::path::Path;

use anyhow::{Context, Result};

use assistant_core::MemoryLoader;
use assistant_core::types::agent::AssistantConfig;

pub fn cmd_reset(db_path: &Path, config: &AssistantConfig, skip_confirm: bool) -> Result<()> {
    let loader = MemoryLoader::new(config);

    // Collect everything that will be removed so we can show the user upfront.
    let memory_files = [
        loader.soul_path().to_path_buf(),
        loader.identity_path().to_path_buf(),
        loader.user_path().to_path_buf(),
        loader.memory_path().to_path_buf(),
    ];

    println!("This will permanently delete:\n");
    println!("  Database : {}", db_path.display());
    for p in &memory_files {
        println!("  Memory   : {}", p.display());
    }
    let notes_dir = loader.notes_dir_path().to_path_buf();
    println!("  Notes dir: {}", notes_dir.display());
    println!();

    if !skip_confirm {
        print!("Are you sure? [y/N] ");
        io::stdout().flush().ok();
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).ok();
        if !matches!(buf.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Delete the SQLite database file.
    if db_path.exists() {
        std::fs::remove_file(db_path)
            .with_context(|| format!("Failed to remove database at {}", db_path.display()))?;
        println!("Removed: {}", db_path.display());
    }

    // Delete the four core memory files.
    for p in &memory_files {
        if p.exists() {
            std::fs::remove_file(p).with_context(|| format!("Failed to remove {}", p.display()))?;
            println!("Removed: {}", p.display());
        }
    }

    // Delete the daily notes directory.
    if notes_dir.exists() {
        std::fs::remove_dir_all(&notes_dir)
            .with_context(|| format!("Failed to remove {}", notes_dir.display()))?;
        println!("Removed: {}", notes_dir.display());
    }

    // Re-seed default memory files so the next session starts with sensible
    // content rather than an empty directory.
    loader.ensure_defaults();
    println!("\nDefaults restored. Assistant is ready for a fresh start.");

    Ok(())
}
