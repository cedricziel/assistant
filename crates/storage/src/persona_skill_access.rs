//! Per-persona skill access control — mode and skill list management.
//!
//! Each persona operates in one of three modes:
//! - `"all"` (default): every skill in the registry is available
//! - `"whitelist"`: only explicitly listed skills are available
//! - `"blacklist"`: all skills except explicitly listed ones are available

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::SqlitePool;

/// Valid persona skill access modes.
pub const MODE_ALL: &str = "all";
pub const MODE_WHITELIST: &str = "whitelist";
pub const MODE_BLACKLIST: &str = "blacklist";

fn validate_mode(mode: &str) -> Result<()> {
    match mode {
        MODE_ALL | MODE_WHITELIST | MODE_BLACKLIST => Ok(()),
        other => anyhow::bail!(
            "Invalid skill access mode '{}'. Expected one of: all, whitelist, blacklist",
            other
        ),
    }
}

/// Trait-based interface for persona skill access configuration.
#[async_trait]
pub trait PersonaSkillAccessStore: Send + Sync {
    async fn get_mode(&self, persona_id: &str) -> Result<String>;
    async fn set_mode(&self, persona_id: &str, mode: &str) -> Result<()>;
    async fn list_skill_names(&self, persona_id: &str) -> Result<Vec<String>>;
    async fn add_skill(&self, persona_id: &str, skill_name: &str) -> Result<()>;
    async fn remove_skill(&self, persona_id: &str, skill_name: &str) -> Result<()>;
    async fn has_skill_list_entries(&self, persona_id: &str) -> Result<bool>;
}

/// SQLite-backed store for per-persona skill access configuration.
pub struct SqlitePersonaSkillAccessStore {
    pool: SqlitePool,
}

impl SqlitePersonaSkillAccessStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PersonaSkillAccessStore for SqlitePersonaSkillAccessStore {
    /// Return the skill access mode for the given persona.
    /// Returns `"all"` if the persona is not found (safe default).
    async fn get_mode(&self, persona_id: &str) -> Result<String> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT skill_access_mode FROM personas WHERE id = ?1")
                .bind(persona_id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to query persona skill_access_mode")?;

        Ok(row.map(|(m,)| m).unwrap_or_else(|| MODE_ALL.to_string()))
    }

    /// Set the skill access mode for a persona.
    async fn set_mode(&self, persona_id: &str, mode: &str) -> Result<()> {
        validate_mode(mode)?;

        let rows_affected =
            sqlx::query("UPDATE personas SET skill_access_mode = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2")
                .bind(mode)
                .bind(persona_id)
                .execute(&self.pool)
                .await
                .context("Failed to update persona skill_access_mode")?
                .rows_affected();

        if rows_affected == 0 {
            anyhow::bail!("Persona '{}' not found", persona_id);
        }
        Ok(())
    }

    /// Return all skill names in the persona's skill list (used for whitelist or blacklist).
    async fn list_skill_names(&self, persona_id: &str) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT skill_name FROM persona_skill_list WHERE persona_id = ?1 ORDER BY skill_name",
        )
        .bind(persona_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query persona_skill_list")?;

        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    /// Add a skill name to the persona's skill list.
    /// No-op if already present (INSERT OR IGNORE).
    async fn add_skill(&self, persona_id: &str, skill_name: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO persona_skill_list (persona_id, skill_name) VALUES (?1, ?2)",
        )
        .bind(persona_id)
        .bind(skill_name)
        .execute(&self.pool)
        .await
        .context("Failed to insert into persona_skill_list")?;

        Ok(())
    }

    /// Remove a skill name from the persona's skill list.
    /// Silent no-op if not present.
    async fn remove_skill(&self, persona_id: &str, skill_name: &str) -> Result<()> {
        sqlx::query("DELETE FROM persona_skill_list WHERE persona_id = ?1 AND skill_name = ?2")
            .bind(persona_id)
            .bind(skill_name)
            .execute(&self.pool)
            .await
            .context("Failed to delete from persona_skill_list")?;

        Ok(())
    }

    /// Return true if the persona has any entries in its skill list.
    async fn has_skill_list_entries(&self, persona_id: &str) -> Result<bool> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM persona_skill_list WHERE persona_id = ?1")
                .bind(persona_id)
                .fetch_one(&self.pool)
                .await
                .context("Failed to count persona_skill_list entries")?;

        Ok(count.0 > 0)
    }
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

#[derive(Default)]
struct PersonaSkillAccessMemoryState {
    modes: HashMap<String, String>,
    skills: HashMap<String, HashSet<String>>,
}

/// In-memory [`PersonaSkillAccessStore`].
pub struct InMemoryPersonaSkillAccessStore {
    state: Arc<Mutex<PersonaSkillAccessMemoryState>>,
}

impl InMemoryPersonaSkillAccessStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PersonaSkillAccessMemoryState::default())),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PersonaSkillAccessMemoryState> {
        match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

impl Default for InMemoryPersonaSkillAccessStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PersonaSkillAccessStore for InMemoryPersonaSkillAccessStore {
    async fn get_mode(&self, persona_id: &str) -> Result<String> {
        let state = self.lock();
        Ok(state
            .modes
            .get(persona_id)
            .cloned()
            .unwrap_or_else(|| MODE_ALL.to_string()))
    }

    async fn set_mode(&self, persona_id: &str, mode: &str) -> Result<()> {
        validate_mode(mode)?;
        let mut state = self.lock();
        state.modes.insert(persona_id.to_string(), mode.to_string());
        Ok(())
    }

    async fn list_skill_names(&self, persona_id: &str) -> Result<Vec<String>> {
        let state = self.lock();
        let mut names: Vec<String> = state
            .skills
            .get(persona_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        names.sort();
        Ok(names)
    }

    async fn add_skill(&self, persona_id: &str, skill_name: &str) -> Result<()> {
        let mut state = self.lock();
        state
            .skills
            .entry(persona_id.to_string())
            .or_default()
            .insert(skill_name.to_string());
        Ok(())
    }

    async fn remove_skill(&self, persona_id: &str, skill_name: &str) -> Result<()> {
        let mut state = self.lock();
        if let Some(set) = state.skills.get_mut(persona_id) {
            set.remove(skill_name);
        }
        Ok(())
    }

    async fn has_skill_list_entries(&self, persona_id: &str) -> Result<bool> {
        let state = self.lock();
        Ok(state
            .skills
            .get(persona_id)
            .map(|s| !s.is_empty())
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;
    use crate::personas::PersonaStore as _;

    #[tokio::test]
    async fn test_default_mode_is_all() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = SqlitePersonaSkillAccessStore::new(storage.pool.clone());

        // Non-existent persona falls back to "all"
        let mode = store.get_mode("nonexistent").await.unwrap();
        assert_eq!(mode, MODE_ALL);
    }

    #[tokio::test]
    async fn test_set_and_get_mode() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        // Ensure default persona exists
        storage
            .persona_store()
            .ensure_exists("default")
            .await
            .unwrap();

        let store = SqlitePersonaSkillAccessStore::new(storage.pool.clone());
        store.set_mode("default", MODE_BLACKLIST).await.unwrap();
        let mode = store.get_mode("default").await.unwrap();
        assert_eq!(mode, MODE_BLACKLIST);
    }

    #[tokio::test]
    async fn test_invalid_mode_rejected() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        storage
            .persona_store()
            .ensure_exists("default")
            .await
            .unwrap();

        let store = SqlitePersonaSkillAccessStore::new(storage.pool.clone());
        let result = store.set_mode("default", "invalid").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_remove_skill() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        storage
            .persona_store()
            .ensure_exists("default")
            .await
            .unwrap();

        let store = SqlitePersonaSkillAccessStore::new(storage.pool.clone());
        store.add_skill("default", "git-commit").await.unwrap();
        store.add_skill("default", "web-search").await.unwrap();

        let names = store.list_skill_names("default").await.unwrap();
        assert_eq!(names, vec!["git-commit", "web-search"]);

        store.remove_skill("default", "git-commit").await.unwrap();
        let names = store.list_skill_names("default").await.unwrap();
        assert_eq!(names, vec!["web-search"]);
    }

    #[tokio::test]
    async fn test_add_skill_idempotent() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        storage
            .persona_store()
            .ensure_exists("default")
            .await
            .unwrap();

        let store = SqlitePersonaSkillAccessStore::new(storage.pool.clone());
        store.add_skill("default", "git-commit").await.unwrap();
        store.add_skill("default", "git-commit").await.unwrap(); // no-op

        let names = store.list_skill_names("default").await.unwrap();
        assert_eq!(names.len(), 1);
    }
}
