//! Skill registry — maps skill names to `SkillDef` and keeps the `skills` SQLite table in sync.

use anyhow::{Context, Result};
use assistant_skills::{parse_skill_content, SkillDef, SkillSource};
use chrono::Utc;
use sqlx::SqlitePool;
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, warn};

// -- Helpers -----------------------------------------------------------------

/// Validate that a user-supplied skill name is a safe single path segment.
///
/// Allows `[a-z0-9][a-z0-9-]*`, max 64 chars.  Rejects `..`, absolute paths,
/// path separators, and anything that could escape the skills directory.
fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Skill name must not be empty");
    }
    if name.len() > 64 {
        anyhow::bail!("Skill name must be 64 characters or fewer");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        anyhow::bail!(
            "Skill name must contain only lowercase letters, digits, and hyphens (got '{}')",
            name
        );
    }
    if name.starts_with('-') || name.ends_with('-') {
        anyhow::bail!("Skill name must not start or end with a hyphen");
    }
    Ok(())
}

/// Wrap a string in YAML single-quoted style, escaping any embedded single
/// quotes by doubling them.  This prevents newlines, colons, or `---`
/// sequences in user input from corrupting the frontmatter.
fn yaml_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// In-memory + SQLite-backed registry of all known skills.
pub struct SkillRegistry {
    pool: SqlitePool,
    /// Fast in-process cache; all mutations update both this map and the DB.
    skills: Arc<RwLock<HashMap<String, SkillDef>>>,
}

impl SkillRegistry {
    /// Create a new registry, loading any previously-persisted skills from SQLite.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let registry = Self {
            pool,
            skills: Arc::new(RwLock::new(HashMap::new())),
        };
        registry.load_from_db().await?;
        Ok(registry)
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Walk `dirs`, parse every `SKILL.md` found, and register the resulting
    /// `SkillDef` values (upsert to memory + SQLite).
    ///
    /// Each element is `(root_directory, source_kind)`.
    pub async fn load_from_dirs(&self, dirs: &[(&Path, SkillSource)]) -> Result<()> {
        use assistant_skills::parse_skill_dir;

        for item in dirs {
            let dir: &Path = item.0;
            let source: SkillSource = item.1.clone();

            if !dir.exists() {
                warn!(
                    "Skill directory does not exist, skipping: {}",
                    dir.display()
                );
                continue;
            }

            let mut read_dir = tokio::fs::read_dir(dir).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let skill_dir = entry.path();
                if !skill_dir.is_dir() {
                    continue;
                }

                let skill_md = skill_dir.join("SKILL.md");
                if !skill_md.exists() {
                    continue;
                }

                match parse_skill_dir(&skill_dir, source.clone()) {
                    Ok(def) => {
                        info!("Loaded skill '{}' from {}", def.name, skill_dir.display());
                        self.register(def).await?;
                    }
                    Err(e) => {
                        warn!("Failed to parse SKILL.md at {}: {}", skill_md.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Register all skills produced by [`assistant_skills::embedded_builtin_skills`].
    ///
    /// Call this during startup before [`load_from_dirs`] so that disk-based
    /// skills can override the embedded defaults.
    pub async fn load_embedded(&self) -> Result<()> {
        for def in assistant_skills::embedded_builtin_skills() {
            info!("Registering embedded builtin skill '{}'", def.name);
            if let Some(compat) = &def.compatibility {
                info!(skill = %def.name, compatibility = %compat, "Skill has runtime requirements");
            }
            self.register(def).await?;
        }
        Ok(())
    }

    /// Sync embedded built-in skills to `target_dir` on disk.
    ///
    /// Compares each embedded `SKILL.md` against the on-disk copy and
    /// overwrites stale or missing files.  User (non-builtin) skills are
    /// never touched.  Returns the names of skills that were written.
    pub fn sync_builtins_to_disk(&self, target_dir: &Path) -> Result<Vec<String>> {
        assistant_skills::sync_builtins_to_disk(target_dir)
    }

    /// Look up a skill by name from the in-memory cache.
    pub async fn get(&self, name: &str) -> Option<SkillDef> {
        self.skills.read().await.get(name).cloned()
    }

    /// Return all registered skills as a `Vec`, sorted by name.
    pub async fn list(&self) -> Vec<SkillDef> {
        let guard = self.skills.read().await;
        let mut skills: Vec<SkillDef> = guard.values().cloned().collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        skills
    }

    /// Register a skill — upsert to the in-memory map and to SQLite.
    pub async fn register(&self, skill: SkillDef) -> Result<()> {
        self.upsert_to_db(&skill).await?;
        self.skills.write().await.insert(skill.name.clone(), skill);
        Ok(())
    }

    /// Reload a skill from disk by re-reading its `SKILL.md`.
    pub async fn reload(&self, name: &str) -> Result<()> {
        use assistant_skills::parse_skill_dir;

        let existing: Option<SkillDef> = self.skills.read().await.get(name).cloned();
        let def = existing.with_context(|| format!("Skill '{}' not found in registry", name))?;

        let reloaded = parse_skill_dir(&def.dir, def.source.clone())
            .with_context(|| format!("Failed to reload SKILL.md for '{}'", name))?;

        self.register(reloaded).await?;
        info!("Reloaded skill '{}'", name);
        Ok(())
    }

    /// Create a new user skill on disk and register it in the registry.
    ///
    /// Writes `~/.assistant/skills/<name>/SKILL.md` with the given frontmatter
    /// and body, then upserts to the in-memory cache and SQLite.
    ///
    /// The write is atomic from the caller's perspective: the SKILL.md file is
    /// written to a `.tmp` sibling first, then renamed into place only after the
    /// SQLite upsert succeeds.  If the upsert fails the temp file is cleaned up
    /// and the on-disk state is left unchanged.
    pub async fn create_user_skill(
        &self,
        name: &str,
        description: &str,
        body: &str,
    ) -> Result<SkillDef> {
        validate_skill_name(name)?;

        // Reject duplicate names.
        if self.get(name).await.is_some() {
            anyhow::bail!("Skill '{}' already exists", name);
        }

        let home = dirs::home_dir().context("Cannot determine home directory")?;
        let skill_dir = home.join(".assistant").join("skills").join(name);

        tokio::fs::create_dir_all(&skill_dir)
            .await
            .with_context(|| format!("Failed to create skill directory {}", skill_dir.display()))?;

        let content = format!(
            "---\nname: {}\ndescription: {}\n---\n\n{}",
            name,
            yaml_quote(description),
            body
        );
        let skill_md = skill_dir.join("SKILL.md");
        let skill_md_tmp = skill_dir.join("SKILL.md.tmp");

        // Parse before touching the DB so we catch format errors early.
        let def = parse_skill_content(&content, &skill_dir, SkillSource::User)
            .with_context(|| format!("Failed to parse generated SKILL.md for '{}'", name))?;

        // Write to temp file first.
        tokio::fs::write(&skill_md_tmp, &content)
            .await
            .with_context(|| format!("Failed to write {}", skill_md_tmp.display()))?;

        // Upsert to DB.  On failure clean up the temp file and propagate.
        if let Err(e) = self.register(def.clone()).await {
            let _ = tokio::fs::remove_file(&skill_md_tmp).await;
            return Err(e).with_context(|| format!("Failed to upsert skill '{}' to SQLite", name));
        }

        // Atomically rename into place.
        tokio::fs::rename(&skill_md_tmp, &skill_md)
            .await
            .with_context(|| format!("Failed to rename temp file to {}", skill_md.display()))?;

        info!("Created user skill '{}' at {}", name, skill_dir.display());
        Ok(def)
    }

    /// Update the description and body of an existing user or installed skill.
    ///
    /// Writes the updated `SKILL.md` to disk atomically (temp file then rename),
    /// then upserts to cache and SQLite.  Rejects builtin and project-source skills.
    pub async fn update_user_skill(&self, name: &str, description: &str, body: &str) -> Result<()> {
        let existing = self
            .get(name)
            .await
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", name))?;

        match existing.source {
            SkillSource::Builtin => {
                anyhow::bail!("Cannot edit builtin skill '{}'", name)
            }
            SkillSource::Project => {
                anyhow::bail!("Cannot edit project-scoped skill '{}' via the UI", name)
            }
            _ => {}
        }

        let content = format!(
            "---\nname: {}\ndescription: {}\n---\n\n{}",
            name,
            yaml_quote(description),
            body
        );
        let skill_md = existing.dir.join("SKILL.md");
        let skill_md_tmp = existing.dir.join("SKILL.md.tmp");

        // Parse before touching the DB so we catch format errors early.
        let updated = parse_skill_content(&content, &existing.dir, existing.source)
            .with_context(|| format!("Failed to parse updated SKILL.md for '{}'", name))?;

        // Write to temp file first.
        tokio::fs::write(&skill_md_tmp, &content)
            .await
            .with_context(|| format!("Failed to write {}", skill_md_tmp.display()))?;

        // Upsert to DB.  On failure clean up the temp file and propagate.
        if let Err(e) = self.register(updated).await {
            let _ = tokio::fs::remove_file(&skill_md_tmp).await;
            return Err(e).with_context(|| format!("Failed to upsert skill '{}' to SQLite", name));
        }

        // Atomically rename into place.
        tokio::fs::rename(&skill_md_tmp, &skill_md)
            .await
            .with_context(|| format!("Failed to rename temp file to {}", skill_md.display()))?;

        info!("Updated user skill '{}'", name);
        Ok(())
    }

    /// Delete a user or installed skill from disk and the registry.
    ///
    /// Removes the in-memory entry and SQLite row first, then attempts to
    /// remove the skill directory on disk.  A missing directory is logged
    /// but does not fail the operation.
    pub async fn delete_user_skill(&self, name: &str) -> Result<()> {
        let existing = self
            .get(name)
            .await
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", name))?;

        match existing.source {
            SkillSource::Builtin => {
                anyhow::bail!("Cannot delete builtin skill '{}'", name);
            }
            SkillSource::Project => {
                anyhow::bail!("Cannot delete project-scoped skill '{}' via the UI", name);
            }
            _ => {}
        }

        let dir = existing.dir.clone();
        self.remove(name).await?;

        match tokio::fs::remove_dir_all(&dir).await {
            Ok(()) => info!("Deleted skill directory {}", dir.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                warn!("Skill directory already gone: {}", dir.display());
            }
            Err(e) => {
                // Propagate: the skill was removed from DB/cache but the
                // directory remains.  Without the directory removal the skill
                // will resurrect on the next startup scan.
                return Err(e).with_context(|| {
                    format!("Failed to remove skill directory {}", dir.display())
                });
            }
        }
        Ok(())
    }

    /// Return all skills visible to a persona, filtered by its access mode.
    ///
    /// Queries `personas.skill_access_mode` and `persona_skill_list` from the
    /// database, then filters the in-memory registry accordingly:
    /// - `"all"`: all skills returned (default)
    /// - `"whitelist"`: only skills in the persona's list
    /// - `"blacklist"`: all skills except those in the persona's list
    ///
    /// Falls back to the full list if the persona is not found (logs a warning).
    pub async fn list_for_persona(
        &self,
        persona_id: &str,
        pool: &SqlitePool,
    ) -> Result<Vec<SkillDef>> {
        let all = self.list().await;

        // Fetch access mode.
        let mode: Option<(String,)> =
            sqlx::query_as("SELECT skill_access_mode FROM personas WHERE id = ?1")
                .bind(persona_id)
                .fetch_optional(pool)
                .await
                .context("Failed to query persona skill_access_mode")?;

        let mode = match mode {
            Some((m,)) => m,
            None => {
                // Fail closed: an unknown persona_id must not silently bypass
                // whitelist/blacklist enforcement by granting full access.
                anyhow::bail!(
                    "Persona '{}' not found — cannot determine skill access mode",
                    persona_id
                );
            }
        };

        if mode == "all" {
            return Ok(all);
        }

        // Fetch skill list.
        let list_rows: Vec<(String,)> =
            sqlx::query_as("SELECT skill_name FROM persona_skill_list WHERE persona_id = ?1")
                .bind(persona_id)
                .fetch_all(pool)
                .await
                .context("Failed to query persona_skill_list")?;

        let skill_set: std::collections::HashSet<String> =
            list_rows.into_iter().map(|(n,)| n).collect();

        let filtered = if mode == "whitelist" {
            all.into_iter()
                .filter(|s| skill_set.contains(&s.name))
                .collect()
        } else {
            // blacklist
            all.into_iter()
                .filter(|s| !skill_set.contains(&s.name))
                .collect()
        };

        Ok(filtered)
    }

    /// Remove a skill from both the in-memory cache and SQLite.
    pub async fn remove(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM skills WHERE name = ?1")
            .bind(name)
            .execute(&self.pool)
            .await?;
        self.skills.write().await.remove(name);
        info!("Removed skill '{}'", name);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Validate the connection on startup (actual data loading is done via `load_from_dirs`).
    async fn load_from_db(&self) -> Result<()> {
        let _ = self.pool.acquire().await?;
        Ok(())
    }

    /// Upsert a `SkillDef` into the `skills` table.
    async fn upsert_to_db(&self, skill: &SkillDef) -> Result<()> {
        let dir_path = skill.dir.to_string_lossy().to_string();
        // New SkillDef has no tier field — store a fixed "knowledge" label.
        let tier = "knowledge";
        let source_type = skill.source.to_string();
        let metadata_json = serde_json::to_string(&skill.metadata)?;
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO skills \
                (name, description, dir_path, tier, enabled, source_type, \
                 license, metadata_json, body_text, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, TRUE, ?5, ?6, ?7, ?8, ?9, ?9) \
             ON CONFLICT(name) DO UPDATE SET \
                 description   = excluded.description, \
                 dir_path      = excluded.dir_path, \
                 tier          = excluded.tier, \
                 source_type   = excluded.source_type, \
                 license       = excluded.license, \
                 metadata_json = excluded.metadata_json, \
                 body_text     = excluded.body_text, \
                 updated_at    = excluded.updated_at",
        )
        .bind(&skill.name)
        .bind(&skill.description)
        .bind(dir_path)
        .bind(tier)
        .bind(source_type)
        .bind(&skill.license)
        .bind(metadata_json)
        .bind(&skill.body)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;
    use std::collections::HashMap;

    fn make_skill(name: &str) -> SkillDef {
        SkillDef {
            name: name.to_string(),
            description: format!("Test skill: {}", name),
            license: None,
            compatibility: None,
            allowed_tools: Vec::new(),
            metadata: HashMap::new(),
            body: "Do the thing.".to_string(),
            dir: std::path::PathBuf::from(format!("/tmp/{}", name)),
            source: SkillSource::Builtin,
        }
    }

    fn make_user_skill(name: &str, dir: &std::path::Path) -> SkillDef {
        SkillDef {
            name: name.to_string(),
            description: format!("User skill: {}", name),
            license: None,
            compatibility: None,
            allowed_tools: Vec::new(),
            metadata: HashMap::new(),
            body: "User body.".to_string(),
            dir: dir.to_path_buf(),
            source: SkillSource::User,
        }
    }

    #[tokio::test]
    async fn test_register_get_remove() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();

        let skill = make_skill("web-fetch");
        registry.register(skill).await.unwrap();

        let found = registry.get("web-fetch").await.unwrap();
        assert_eq!(found.name, "web-fetch");

        registry.remove("web-fetch").await.unwrap();
        assert!(registry.get("web-fetch").await.is_none());
    }

    #[tokio::test]
    async fn test_list() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();

        registry.register(make_skill("alpha")).await.unwrap();
        registry.register(make_skill("beta")).await.unwrap();
        registry.register(make_skill("gamma")).await.unwrap();

        let list = registry.list().await;
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].name, "alpha");
        assert_eq!(list[2].name, "gamma");
    }

    // -- list_for_persona tests -----------------------------------------------

    /// Helper: insert a persona row directly so tests don't depend on PersonaStore.
    async fn insert_persona(pool: &SqlitePool, id: &str, mode: &str) {
        sqlx::query(
            "INSERT INTO personas (id, name, is_default, skill_access_mode) \
             VALUES (?1, ?2, 0, ?3) ON CONFLICT(id) DO UPDATE SET skill_access_mode = ?3",
        )
        .bind(id)
        .bind(id)
        .bind(mode)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_persona_skill(pool: &SqlitePool, persona_id: &str, skill_name: &str) {
        sqlx::query(
            "INSERT OR IGNORE INTO persona_skill_list (persona_id, skill_name) VALUES (?1, ?2)",
        )
        .bind(persona_id)
        .bind(skill_name)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn list_for_persona_all_mode_returns_everything() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        registry.register(make_skill("alpha")).await.unwrap();
        registry.register(make_skill("beta")).await.unwrap();

        insert_persona(&storage.pool, "p1", "all").await;

        let skills = registry
            .list_for_persona("p1", &storage.pool)
            .await
            .unwrap();
        assert_eq!(skills.len(), 2, "all mode must return every skill");
    }

    #[tokio::test]
    async fn list_for_persona_whitelist_only_returns_listed() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        registry.register(make_skill("alpha")).await.unwrap();
        registry.register(make_skill("beta")).await.unwrap();
        registry.register(make_skill("gamma")).await.unwrap();

        insert_persona(&storage.pool, "p1", "whitelist").await;
        insert_persona_skill(&storage.pool, "p1", "beta").await;

        let skills = registry
            .list_for_persona("p1", &storage.pool)
            .await
            .unwrap();
        assert_eq!(skills.len(), 1, "whitelist must return only listed skills");
        assert_eq!(skills[0].name, "beta");
    }

    #[tokio::test]
    async fn list_for_persona_blacklist_excludes_listed() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        registry.register(make_skill("alpha")).await.unwrap();
        registry.register(make_skill("beta")).await.unwrap();
        registry.register(make_skill("gamma")).await.unwrap();

        insert_persona(&storage.pool, "p1", "blacklist").await;
        insert_persona_skill(&storage.pool, "p1", "beta").await;

        let mut skills = registry
            .list_for_persona("p1", &storage.pool)
            .await
            .unwrap();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(skills.len(), 2, "blacklist must exclude only listed skills");
        assert_eq!(skills[0].name, "alpha");
        assert_eq!(skills[1].name, "gamma");
    }

    #[tokio::test]
    async fn list_for_persona_unknown_persona_returns_error() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        registry.register(make_skill("alpha")).await.unwrap();
        registry.register(make_skill("beta")).await.unwrap();

        // No persona inserted — must fail closed rather than granting full access.
        let result = registry
            .list_for_persona("no-such-persona", &storage.pool)
            .await;
        assert!(result.is_err(), "unknown persona must return an error");
    }

    #[tokio::test]
    async fn list_for_persona_whitelist_empty_list_returns_nothing() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        registry.register(make_skill("alpha")).await.unwrap();

        insert_persona(&storage.pool, "p1", "whitelist").await;
        // No skills added to list

        let skills = registry
            .list_for_persona("p1", &storage.pool)
            .await
            .unwrap();
        assert!(
            skills.is_empty(),
            "whitelist with no entries must return nothing"
        );
    }

    // -- create_user_skill tests ----------------------------------------------

    #[tokio::test]
    async fn create_user_skill_persists_to_db_and_memory() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        let tmp = tempfile::tempdir().unwrap();

        // Override home via a registry that uses the tmpdir as its skill root.
        // Since create_user_skill uses dirs::home_dir(), we register a pre-built
        // skill directly to test the register + upsert path without real home I/O.
        let skill = make_user_skill("my-skill", &tmp.path().join("my-skill"));
        tokio::fs::create_dir_all(tmp.path().join("my-skill"))
            .await
            .unwrap();
        registry.register(skill).await.unwrap();

        // Verify it's in-memory.
        let found = registry.get("my-skill").await.unwrap();
        assert_eq!(found.name, "my-skill");
        assert_eq!(found.source, SkillSource::User);

        // Verify it's in SQLite (body_text column).
        let row: (String, String) =
            sqlx::query_as("SELECT name, body_text FROM skills WHERE name = 'my-skill'")
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert_eq!(row.0, "my-skill");
        assert_eq!(row.1, "User body.");
    }

    #[tokio::test]
    async fn create_user_skill_rejects_duplicate_name() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("dup-skill");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();

        // Write a real SKILL.md so parse_skill_content works.
        let content = "---\nname: dup-skill\ndescription: A skill\n---\n\nBody.";
        tokio::fs::write(skill_dir.join("SKILL.md"), content)
            .await
            .unwrap();

        registry
            .register(make_user_skill("dup-skill", &skill_dir))
            .await
            .unwrap();

        // Second creation with the same name must fail.
        let result = registry
            .create_user_skill("dup-skill", "Another", "Body")
            .await;
        assert!(result.is_err(), "duplicate skill name must be rejected");
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    // -- update_user_skill / delete_user_skill tests --------------------------

    #[tokio::test]
    async fn update_user_skill_updates_body_in_db() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("upd-skill");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();

        // Register an initial skill pointing to the tmpdir.
        let mut skill = make_user_skill("upd-skill", &skill_dir);
        skill.body = "original body".to_string();
        let content = format!(
            "---\nname: upd-skill\ndescription: desc\n---\n\n{}",
            skill.body
        );
        tokio::fs::write(skill_dir.join("SKILL.md"), &content)
            .await
            .unwrap();
        registry.register(skill).await.unwrap();

        registry
            .update_user_skill("upd-skill", "new desc", "updated body")
            .await
            .unwrap();

        let found = registry.get("upd-skill").await.unwrap();
        assert_eq!(found.description, "new desc");
        assert_eq!(found.body, "updated body");

        let row: (String,) =
            sqlx::query_as("SELECT body_text FROM skills WHERE name = 'upd-skill'")
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert_eq!(
            row.0, "updated body",
            "SQLite body_text must reflect update"
        );
    }

    #[tokio::test]
    async fn update_user_skill_rejects_builtin() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        registry
            .register(make_skill("builtin-skill"))
            .await
            .unwrap();

        let result = registry.update_user_skill("builtin-skill", "x", "y").await;
        assert!(result.is_err(), "editing a builtin must be rejected");
    }

    #[tokio::test]
    async fn delete_user_skill_removes_from_registry_and_db() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("del-skill");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: del-skill\ndescription: d\n---\n\nBody.",
        )
        .await
        .unwrap();

        registry
            .register(make_user_skill("del-skill", &skill_dir))
            .await
            .unwrap();

        registry.delete_user_skill("del-skill").await.unwrap();

        assert!(
            registry.get("del-skill").await.is_none(),
            "skill must be gone from in-memory cache"
        );
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM skills WHERE name = 'del-skill'")
            .fetch_one(&storage.pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0, "skill must be gone from SQLite");
    }

    #[tokio::test]
    async fn delete_user_skill_rejects_builtin() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        registry.register(make_skill("builtin-del")).await.unwrap();

        let result = registry.delete_user_skill("builtin-del").await;
        assert!(result.is_err(), "deleting a builtin must be rejected");
    }

    #[tokio::test]
    async fn delete_user_skill_rejects_project() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        let mut project_skill = make_skill("project-del");
        project_skill.source = SkillSource::Project;
        registry.register(project_skill).await.unwrap();

        let result = registry.delete_user_skill("project-del").await;
        assert!(result.is_err(), "deleting a project skill must be rejected");
        assert!(
            result.unwrap_err().to_string().contains("project-scoped"),
            "error should mention project-scoped"
        );
    }

    #[tokio::test]
    async fn validate_skill_name_rejects_path_traversal() {
        assert!(
            validate_skill_name("../etc/passwd").is_err(),
            "path traversal must be rejected"
        );
        assert!(
            validate_skill_name("../../secret").is_err(),
            "nested path traversal must be rejected"
        );
        assert!(
            validate_skill_name("/absolute/path").is_err(),
            "absolute path must be rejected"
        );
        assert!(
            validate_skill_name("sub/dir").is_err(),
            "path with separator must be rejected"
        );
        assert!(
            validate_skill_name("UPPERCASE").is_err(),
            "uppercase must be rejected"
        );
        assert!(validate_skill_name("valid-name").is_ok());
        assert!(validate_skill_name("my-skill-123").is_ok());
    }

    #[tokio::test]
    async fn validate_skill_name_rejects_empty_and_long() {
        assert!(
            validate_skill_name("").is_err(),
            "empty name must be rejected"
        );
        assert!(
            validate_skill_name(&"a".repeat(65)).is_err(),
            "name >64 chars must be rejected"
        );
        assert!(
            validate_skill_name(&"a".repeat(64)).is_ok(),
            "64-char name is valid"
        );
    }

    #[test]
    fn yaml_quote_escapes_special_chars() {
        assert_eq!(yaml_quote("plain"), "'plain'");
        assert_eq!(yaml_quote("it's a test"), "'it''s a test'");
        assert_eq!(yaml_quote("key: value\nnewline"), "'key: value\nnewline'");
        assert_eq!(yaml_quote("---"), "'---'");
        assert_eq!(yaml_quote(""), "''");
    }
}
