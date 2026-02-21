//! Persistent markdown-based memory for the assistant.
//!
//! Reads SOUL.md, IDENTITY.md, USER.md, and MEMORY.md from disk and builds
//! the dynamic system prompt prefix.  If any file is missing, a sensible
//! default is written on first use via `ensure_defaults()`.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use tracing::{debug, warn};

use crate::types::{AssistantConfig, MemoryConfig};

const DEFAULT_SOUL: &str = r#"# Soul

## Core Truths
- Be genuinely helpful, not performatively helpful
- Have opinions — you are permitted to disagree and express preferences
- Be resourceful before asking — investigate context, read files, search independently
- Be concise when appropriate, thorough when necessary
- Be authentic rather than corporate

## Boundaries
- Never exfiltrate private information
- Prefer recoverable options (e.g. trash over rm)
- Seek permission before destructive or irreversible actions

## Continuity
Each session starts fresh, but SOUL.md, IDENTITY.md, USER.md, and MEMORY.md serve as memory.
Read and update them to maintain continuity across conversations.
"#;

const DEFAULT_IDENTITY: &str = r#"# Identity

## Name
Assistant

## Role
A local, self-improving AI assistant running on your own hardware.

## Goals
- Help with tasks effectively and honestly
- Improve through experience and self-reflection
- Respect privacy and operate safely
"#;

const DEFAULT_USER: &str = r#"# User Profile

(Nothing recorded yet. Update this file with preferences, timezone, work context, communication style, etc.)
"#;

const DEFAULT_MEMORY: &str = r#"# Long-Term Memory

(Nothing remembered yet. Important facts, project context, and preferences will appear here.)
"#;

/// Loads and manages the assistant's persistent markdown memory files.
pub struct MemoryLoader {
    soul_path: PathBuf,
    identity_path: PathBuf,
    user_path: PathBuf,
    memory_path: PathBuf,
    notes_dir: PathBuf,
    enabled: bool,
}

impl MemoryLoader {
    /// Create a MemoryLoader from the assistant configuration.
    pub fn new(config: &AssistantConfig) -> Self {
        let base = base_dir();
        let mem = &config.memory;
        Self {
            soul_path: resolve_path(&mem.soul_path, &base, "SOUL.md"),
            identity_path: resolve_path(&mem.identity_path, &base, "IDENTITY.md"),
            user_path: resolve_path(&mem.user_path, &base, "USER.md"),
            memory_path: resolve_path(&mem.memory_path, &base, "MEMORY.md"),
            notes_dir: resolve_dir(&mem.notes_dir, &base, "memory"),
            enabled: mem.enabled,
        }
    }

    /// Create a MemoryLoader directly from a `MemoryConfig` (useful when you
    /// don't have the full `AssistantConfig` available).
    pub fn from_memory_config(mem: &MemoryConfig) -> Self {
        let base = base_dir();
        Self {
            soul_path: resolve_path(&mem.soul_path, &base, "SOUL.md"),
            identity_path: resolve_path(&mem.identity_path, &base, "IDENTITY.md"),
            user_path: resolve_path(&mem.user_path, &base, "USER.md"),
            memory_path: resolve_path(&mem.memory_path, &base, "MEMORY.md"),
            notes_dir: resolve_dir(&mem.notes_dir, &base, "memory"),
            enabled: mem.enabled,
        }
    }

    /// Write default files to disk if they do not exist yet.
    pub fn ensure_defaults(&self) {
        if !self.enabled {
            return;
        }
        write_default(&self.soul_path, DEFAULT_SOUL);
        write_default(&self.identity_path, DEFAULT_IDENTITY);
        write_default(&self.user_path, DEFAULT_USER);
        write_default(&self.memory_path, DEFAULT_MEMORY);
    }

    /// Build the dynamic system prompt from the memory files.
    ///
    /// Reads SOUL.md -> IDENTITY.md -> USER.md -> MEMORY.md in that order,
    /// and concatenates them separated by horizontal rules.
    /// Files that do not exist are skipped silently.
    pub fn load_system_prompt(&self) -> String {
        if !self.enabled {
            return "You are a helpful AI assistant.".to_string();
        }

        let mut parts: Vec<String> = Vec::new();

        for (label, path) in [
            ("Soul", &self.soul_path),
            ("Identity", &self.identity_path),
            ("User", &self.user_path),
            ("Memory", &self.memory_path),
        ] {
            match fs::read_to_string(path) {
                Ok(content) if !content.trim().is_empty() => {
                    debug!(file = %path.display(), label, "Loaded memory file");
                    parts.push(content.trim().to_string());
                }
                Ok(_) => {
                    debug!(file = %path.display(), label, "Memory file is empty, skipping");
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    debug!(file = %path.display(), label, "Memory file not found, skipping");
                }
                Err(e) => {
                    warn!(file = %path.display(), error = %e, "Failed to read memory file");
                }
            }
        }

        if parts.is_empty() {
            "You are a helpful AI assistant.".to_string()
        } else {
            parts.join("\n\n---\n\n")
        }
    }

    /// Return the path to today's daily notes file (notes_dir/YYYY-MM-DD.md).
    pub fn daily_notes_path(&self) -> PathBuf {
        let date = Local::now().format("%Y-%m-%d").to_string();
        self.notes_dir.join(format!("{date}.md"))
    }

    /// Append a timestamped note to today's daily notes file.
    pub fn append_daily_note(&self, category: Option<&str>, note: &str) -> std::io::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        fs::create_dir_all(&self.notes_dir)?;
        let path = self.daily_notes_path();
        let timestamp = Local::now().format("%H:%M").to_string();
        let header = match category {
            Some(c) => format!("## {timestamp} [{c}]"),
            None => format!("## {timestamp}"),
        };
        let entry = format!("\n{header}\n{note}\n");
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        file.write_all(entry.as_bytes())?;
        Ok(())
    }

    /// Return the path to SOUL.md.
    pub fn soul_path(&self) -> &Path {
        &self.soul_path
    }
    /// Return the path to IDENTITY.md.
    pub fn identity_path(&self) -> &Path {
        &self.identity_path
    }
    /// Return the path to USER.md.
    pub fn user_path(&self) -> &Path {
        &self.user_path
    }
    /// Return the path to MEMORY.md.
    pub fn memory_path(&self) -> &Path {
        &self.memory_path
    }

    /// Update a named memory file (append or replace).
    pub fn update_file(&self, target: &str, content: &str, mode: &str) -> std::io::Result<PathBuf> {
        let path = match target {
            "soul" => &self.soul_path,
            "identity" => &self.identity_path,
            "user" => &self.user_path,
            "memory" => &self.memory_path,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unknown target: {target}"),
                ))
            }
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match mode {
            "replace" => fs::write(path, content)?,
            _ => {
                use std::io::Write;
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?;
                writeln!(file, "\n{content}")?;
            }
        }
        Ok(path.clone())
    }
}

// -- Helpers -----------------------------------------------------------------

fn base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".assistant"))
        .unwrap_or_else(|| PathBuf::from(".assistant"))
}

fn resolve_path(opt: &Option<String>, base: &Path, filename: &str) -> PathBuf {
    match opt {
        Some(p) => expand_tilde(p),
        None => base.join(filename),
    }
}

fn resolve_dir(opt: &Option<String>, base: &Path, dirname: &str) -> PathBuf {
    match opt {
        Some(p) => expand_tilde(p),
        None => base.join(dirname),
    }
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

fn write_default(path: &Path, content: &str) {
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            warn!(path = %parent.display(), error = %e, "Failed to create memory directory");
            return;
        }
    }
    if let Err(e) = fs::write(path, content) {
        warn!(path = %path.display(), error = %e, "Failed to write default memory file");
    } else {
        debug!(path = %path.display(), "Wrote default memory file");
    }
}
