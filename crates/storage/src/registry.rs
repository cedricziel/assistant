//! Skill registry — maps skill names to `SkillDef` and keeps the `skills` SQLite table in sync.

use anyhow::{Context, Result};
use assistant_skills::{SkillDef, SkillSource};
use chrono::Utc;
use sqlx::SqlitePool;
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, warn};

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
            self.register(def).await?;
        }
        Ok(())
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
                 license, metadata_json, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, TRUE, ?5, ?6, ?7, ?8, ?8) \
             ON CONFLICT(name) DO UPDATE SET \
                 description   = excluded.description, \
                 dir_path      = excluded.dir_path, \
                 tier          = excluded.tier, \
                 source_type   = excluded.source_type, \
                 license       = excluded.license, \
                 metadata_json = excluded.metadata_json, \
                 updated_at    = excluded.updated_at",
        )
        .bind(&skill.name)
        .bind(&skill.description)
        .bind(dir_path)
        .bind(tier)
        .bind(source_type)
        .bind(&skill.license)
        .bind(metadata_json)
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
}
