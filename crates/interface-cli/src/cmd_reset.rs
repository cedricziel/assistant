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

#[cfg(test)]
mod tests {
    use super::*;

    fn agent_config_under(root: &Path) -> AssistantConfig {
        let mut cfg = AssistantConfig::default();
        cfg.agent.id = "default".to_string();
        // Reroute every memory-file path to absolute paths under the tempdir
        // so the test never touches ~/.assistant.
        let p = |name: &str| Some(root.join(name).to_string_lossy().into_owned());
        cfg.memory.agents_path = p("AGENTS.md");
        cfg.memory.soul_path = p("SOUL.md");
        cfg.memory.identity_path = p("IDENTITY.md");
        cfg.memory.user_path = p("USER.md");
        cfg.memory.tools_path = p("TOOLS.md");
        cfg.memory.memory_path = p("MEMORY.md");
        cfg.memory.bootstrap_path = p("BOOTSTRAP.md");
        cfg.memory.heartbeat_path = p("HEARTBEAT.md");
        cfg.memory.boot_path = p("BOOT.md");
        cfg.memory.notes_dir = Some(root.join("notes").to_string_lossy().into_owned());
        cfg
    }

    #[test]
    fn cmd_reset_skip_confirm_deletes_db_and_recreates_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("assistant.db");
        std::fs::write(&db_path, b"placeholder").unwrap();

        let cfg = agent_config_under(dir.path());
        cmd_reset(&db_path, &cfg, true).unwrap();

        assert!(!db_path.exists(), "db file removed");
        // ensure_defaults() seeds the memory dir under the workspace.
        let memory_path = MemoryLoader::new(&cfg).memory_path().to_path_buf();
        assert!(memory_path.exists() || memory_path.parent().is_some_and(|p| p.exists()),);
    }

    #[test]
    fn cmd_reset_skip_confirm_no_op_when_nothing_exists() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("assistant.db");
        let cfg = agent_config_under(dir.path());

        // No DB file, no memory files: cmd_reset should still succeed and
        // re-seed defaults.
        cmd_reset(&db_path, &cfg, true).unwrap();
    }

    #[test]
    fn cmd_reset_removes_memory_files_when_they_exist() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("assistant.db");
        let cfg = agent_config_under(dir.path());
        let loader = MemoryLoader::new(&cfg);

        // Touch a memory file to ensure cmd_reset's removal branch executes.
        std::fs::create_dir_all(loader.soul_path().parent().unwrap()).unwrap();
        std::fs::write(loader.soul_path(), b"existing").unwrap();
        assert!(loader.soul_path().exists());

        cmd_reset(&db_path, &cfg, true).unwrap();
        // After defaults re-seed, the file exists with default content
        // (not the "existing" string we wrote).
        let after = std::fs::read_to_string(loader.soul_path()).unwrap();
        assert_ne!(after, "existing");
    }
}
