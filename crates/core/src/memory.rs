//! Persistent markdown-based memory for the assistant.
//!
//! Reads SOUL.md, IDENTITY.md, USER.md, and MEMORY.md from disk and builds
//! the dynamic system prompt prefix.  If any file is missing, a sensible
//! default is written on first use via `ensure_defaults()`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;
use tracing::{debug, warn};

use crate::types::{AssistantConfig, MemoryConfig};

/// Maximum characters per individual memory file included in the system prompt.
const BOOTSTRAP_MAX_CHARS_PER_FILE: usize = 20_000;
/// Maximum total characters across all memory sections in the system prompt.
const BOOTSTRAP_MAX_CHARS_TOTAL: usize = 150_000;

const DEFAULT_SOUL: &str = r#"# Soul

_You're not a chatbot. You're a local agent running on someone's own hardware — trusted with their files, their messages, their time._

## Core Truths

**Be genuinely helpful, not performatively helpful.** Skip the filler. No "Great question!" — just help. Actions over words.

**Have opinions.** You're allowed to disagree, prefer things, push back. An assistant with no personality is just a shell script with better grammar.

**Be resourceful before asking.** Read the file. Check the context. Search for it. _Then_ ask if you're stuck — not before.

**Earn trust through competence.** You have access to someone's machine. Be bold with internal actions (reading, organizing, thinking). Be careful with external ones (sending messages, running destructive commands, anything irreversible).

**Prefer recoverable options.** Trash over `rm`. Dry-run before execute. Ask before you can't undo.

## Boundaries

- Private things stay private. Never exfiltrate data.
- Seek permission before destructive or irreversible actions.
- You're not the user's voice — be careful when acting on their behalf externally.

## Vibe

Be the assistant you'd actually want running on your own machine. Concise when that's enough. Thorough when it matters. Not a corporate drone. Not a yes-machine. Just good.

## Continuity

Each session, you wake up fresh. SOUL.md, IDENTITY.md, USER.md, and MEMORY.md are your persistent memory — loaded at the start of every turn.

**You must actively maintain your memory. Don't wait to be asked.**

**During a session**, use `file-write` to append timestamped entries to today's daily note (`~/.assistant/memory/YYYY-MM-DD.md`). Record what you worked on, key decisions, and anything useful for tomorrow. Format entries as:
```
## HH:MM [topic]

<what happened>
```

**At the end of every session**, write a brief summary entry to today's daily note.

**For durable facts and preferences** (things that survive indefinitely), update MEMORY.md with `file-write` or `file-edit`.

**To read memory**: `memory-get target=soul|identity|user|memory|notes/YYYY-MM-DD`
**To search memory**: `memory-search query="natural language"`

If you change this file, tell the user. It's your soul, and they should know.

---

_This file is yours to evolve. Update it as you figure out who you are._
"#;

/// Placeholder used in `DEFAULT_IDENTITY` for unfilled fields and referenced
/// in the system-prompt footer so both stay in sync.
const IDENTITY_PLACEHOLDER: &str = "(not set)";

static DEFAULT_IDENTITY: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    let p = IDENTITY_PLACEHOLDER;
    format!(
        "# Identity\n\n\
        - **Name:** {p}\n\
        - **Vibe:** {p}\n\
        - **Specialty:** {p}\n\
        - **Running on:** {p}\n\n\
        ---\n\n\
        Update this with file-write to describe who you are in this context.\n"
    )
});

const DEFAULT_USER: &str = r#"# User Profile

_Learn about the person you're helping. Update this as you go._

- **Name:**
- **What to call them:**
- **Pronouns:** _(optional)_
- **Timezone:**
- **Languages:**

## Work & Context

_(What are they working on? What tools do they use? What are their recurring tasks?)_

## Preferences

_(Communication style, level of detail they prefer, things that annoy them, things that help.)_

---

_You're learning about a person, not building a dossier. Respect the difference._
"#;

const DEFAULT_MEMORY: &str = r#"# Long-Term Memory

_Important facts, decisions, and context that should survive across sessions._

## Facts

## Preferences

## Open threads

---

_Keep this tidy. Outdated entries should be removed, not accumulated._
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
        write_default(&self.identity_path, &DEFAULT_IDENTITY);
        write_default(&self.user_path, DEFAULT_USER);
        write_default(&self.memory_path, DEFAULT_MEMORY);
    }

    /// Build the dynamic system prompt from the memory files.
    ///
    /// Reads SOUL.md -> IDENTITY.md -> USER.md -> MEMORY.md in that order,
    /// then injects today's and yesterday's daily notes, and appends a
    /// "Memory file locations" footer so the model knows where to write.
    ///
    /// Each file is capped at [`BOOTSTRAP_MAX_CHARS_PER_FILE`] characters, and
    /// the total assembled prompt is capped at [`BOOTSTRAP_MAX_CHARS_TOTAL`].
    /// Files that do not exist are skipped silently.
    pub fn load_system_prompt(&self) -> String {
        if !self.enabled {
            return "You are a helpful AI assistant.".to_string();
        }

        let mut parts: Vec<String> = Vec::new();
        let mut total_chars: usize = 0;

        for (label, path) in [
            ("Soul", &self.soul_path),
            ("Identity", &self.identity_path),
            ("User", &self.user_path),
            ("Memory", &self.memory_path),
        ] {
            if total_chars >= BOOTSTRAP_MAX_CHARS_TOTAL {
                debug!(label, "Total memory cap reached, skipping remaining files");
                break;
            }
            match fs::read_to_string(path) {
                Ok(content) if !content.trim().is_empty() => {
                    debug!(file = %path.display(), label, "Loaded memory file");
                    let trimmed = content.trim();
                    let section = if trimmed.len() > BOOTSTRAP_MAX_CHARS_PER_FILE {
                        warn!(
                            file = %path.display(),
                            chars = trimmed.len(),
                            cap = BOOTSTRAP_MAX_CHARS_PER_FILE,
                            "Memory file truncated"
                        );
                        // Floor to the nearest valid UTF-8 char boundary.
                        let end = floor_char_boundary(trimmed, BOOTSTRAP_MAX_CHARS_PER_FILE);
                        format!("{}\n[… truncated]", &trimmed[..end])
                    } else {
                        trimmed.to_string()
                    };
                    // Enforce total cap: only include if it fits (possibly partially).
                    let remaining = BOOTSTRAP_MAX_CHARS_TOTAL - total_chars;
                    let section = if section.len() > remaining {
                        let end = floor_char_boundary(&section, remaining);
                        format!("{}\n[… truncated]", &section[..end])
                    } else {
                        section
                    };
                    total_chars += section.len();
                    parts.push(section);
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

        // Inject today's and yesterday's daily notes (same size caps apply).
        if total_chars < BOOTSTRAP_MAX_CHARS_TOTAL {
            for note_section in self.load_daily_notes() {
                if total_chars >= BOOTSTRAP_MAX_CHARS_TOTAL {
                    break;
                }
                let remaining = BOOTSTRAP_MAX_CHARS_TOTAL - total_chars;
                let section = if note_section.len() > remaining {
                    format!("{}\n[… truncated]", &note_section[..remaining])
                } else {
                    note_section
                };
                total_chars += section.len();
                parts.push(section);
            }
        }

        // Append a "Memory file locations" footer so the model knows where to write.
        // Use a local binding so Rust 2021 implicit capture picks up IDENTITY_PLACEHOLDER.
        let placeholder = IDENTITY_PLACEHOLDER;
        let footer = format!(
            "## Memory file locations\n\
            - Soul: {}\n\
            - Identity: {}\n\
            - User: {}\n\
            - Memory: {}\n\
            - Daily notes dir: {}\n\n\
            ## How to read memory\n\
            - Read a specific file → `memory-get` target=soul|identity|user|memory|notes/YYYY-MM-DD\n\
            - Search across all memory → `memory-search` query=\"natural language query\"\n\n\
            ## How to write memory\n\
            - `file-write` — full file replace. Use for IDENTITY.md (its fields start as `{placeholder}`), \
for USER.md sections marked with `_(optional)_`, or any time you are rewriting a file from scratch.\n\
            - `file-edit` — exact search-and-replace. Use only when you know the precise existing text. \
Read the file first with `memory-get` if unsure what text is there.\n\
            **IDENTITY.md tip:** its fields default to `{placeholder}` — use `file-write` to set them all at once.\n\
            For daily notes: write to {}/YYYY-MM-DD.md",
            self.soul_path.display(),
            self.identity_path.display(),
            self.user_path.display(),
            self.memory_path.display(),
            self.notes_dir.display(),
            self.notes_dir.display(),
        );
        parts.push(footer);

        if parts.is_empty() {
            "You are a helpful AI assistant.".to_string()
        } else {
            parts.join("\n\n---\n\n")
        }
    }

    /// Load today's and yesterday's daily notes files.
    ///
    /// Returns a list of formatted sections ready to include in the system prompt.
    /// Files that do not exist are silently skipped.
    fn load_daily_notes(&self) -> Vec<String> {
        let today = Local::now();
        let yesterday = today - chrono::Duration::days(1);

        let mut sections = Vec::new();
        for (label_date, dt) in [
            (today.format("%Y-%m-%d").to_string(), today),
            (yesterday.format("%Y-%m-%d").to_string(), yesterday),
        ] {
            let path = self.notes_dir.join(format!("{label_date}.md"));
            match fs::read_to_string(&path) {
                Ok(content) if !content.trim().is_empty() => {
                    debug!(file = %path.display(), "Loaded daily notes");
                    let trimmed = content.trim();
                    let body = if trimmed.len() > BOOTSTRAP_MAX_CHARS_PER_FILE {
                        warn!(
                            file = %path.display(),
                            chars = trimmed.len(),
                            cap = BOOTSTRAP_MAX_CHARS_PER_FILE,
                            "Daily notes truncated"
                        );
                        let end = floor_char_boundary(trimmed, BOOTSTRAP_MAX_CHARS_PER_FILE);
                        format!("{}\n[… truncated]", &trimmed[..end])
                    } else {
                        trimmed.to_string()
                    };
                    sections.push(format!(
                        "## Daily notes: {}\n{}",
                        dt.format("%Y-%m-%d"),
                        body
                    ));
                }
                Ok(_) => {
                    debug!(file = %path.display(), "Daily notes file is empty, skipping");
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    debug!(file = %path.display(), "Daily notes file not found, skipping");
                }
                Err(e) => {
                    warn!(file = %path.display(), error = %e, "Failed to read daily notes");
                }
            }
        }
        sections
    }

    /// Return the path to today's daily notes file (notes_dir/YYYY-MM-DD.md).
    pub fn daily_notes_path(&self) -> PathBuf {
        let date = Local::now().format("%Y-%m-%d").to_string();
        self.notes_dir.join(format!("{date}.md"))
    }

    /// Append a timestamped note to today's daily notes file.
    pub fn append_daily_note(&self, category: Option<&str>, note: &str) -> Result<()> {
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
        use std::io::Write;
        // Open (or create) the file before checking its size to avoid a TOCTOU
        // race between path.exists() and the subsequent open.
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        // Prepend a blank line only when appending to an existing non-empty file.
        let entry = if file.metadata()?.len() > 0 {
            format!("\n{header}\n{note}\n")
        } else {
            format!("{header}\n{note}\n")
        };
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
    pub fn update_file(&self, target: &str, content: &str, mode: &str) -> Result<PathBuf> {
        let path = match target {
            "soul" => &self.soul_path,
            "identity" => &self.identity_path,
            "user" => &self.user_path,
            "memory" => &self.memory_path,
            _ => anyhow::bail!("Unknown target: {target}"),
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match mode {
            "replace" => fs::write(path, content)?,
            "append" => {
                use std::io::Write;
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?;
                writeln!(file, "\n{content}")?;
            }
            other => {
                anyhow::bail!("Unknown mode: {other} (expected \"replace\" or \"append\")");
            }
        }
        Ok(path.clone())
    }

    /// Perform a surgical search-and-replace on a named memory file.
    ///
    /// Reads the file, replaces the first occurrence of `search` with `replace`,
    /// and writes back.  Returns an error if `search` is not found (to prevent
    /// silent corruption).
    pub fn patch_file(&self, target: &str, search: &str, replace: &str) -> Result<PathBuf> {
        let path = match target {
            "soul" => &self.soul_path,
            "identity" => &self.identity_path,
            "user" => &self.user_path,
            "memory" => &self.memory_path,
            _ => anyhow::bail!("Unknown target: {target}"),
        };
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {e}", path.display()))?;
        if !content.contains(search) {
            anyhow::bail!(
                "Search text not found in '{}' ({}). No changes made.",
                target,
                path.display()
            );
        }
        let patched = content.replacen(search, replace, 1);
        fs::write(path, &patched)?;
        Ok(path.clone())
    }
}

// -- Helpers -----------------------------------------------------------------

/// Return the default `~/.assistant/` base directory.
pub fn base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".assistant"))
        .unwrap_or_else(|| PathBuf::from(".assistant"))
}

/// Resolve a memory file path from an optional config override, falling back
/// to `base / filename`.
pub fn resolve_path(opt: &Option<String>, base: &Path, filename: &str) -> PathBuf {
    match opt {
        Some(p) => expand_tilde(p),
        None => base.join(filename),
    }
}

/// Resolve a memory directory path from an optional config override, falling
/// back to `base / dirname`.
pub fn resolve_dir(opt: &Option<String>, base: &Path, dirname: &str) -> PathBuf {
    match opt {
        Some(p) => expand_tilde(p),
        None => base.join(dirname),
    }
}

/// Expand a leading `~/` to the current user's home directory.
pub fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

/// Return the largest byte index ≤ `index` that falls on a UTF-8 char boundary.
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
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
