//! Saved tool permission rules backed by the `tool_permission_rules` table.
//!
//! A user saves a rule from an approval prompt ("allow always"). The permission
//! pipeline reads the owner's rules as its user stages. Admins list and revoke
//! every member's rules through [`ToolPermissionRuleStore::list_for_space`] and
//! [`ToolPermissionRuleStore::delete`] without an owner filter.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use assistant_core::clock::{Clock, SystemClock};

/// Whether a saved rule permits or forbids the tool calls it matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleEffect {
    Allow,
    Deny,
}

impl RuleEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        match s {
            "allow" => Ok(Self::Allow),
            "deny" => Ok(Self::Deny),
            other => anyhow::bail!("unknown rule effect '{other}'"),
        }
    }
}

/// One saved rule, owned by a user.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolPermissionRule {
    pub id: Uuid,
    pub user_id: String,
    pub effect: RuleEffect,
    /// Rule text in the permission grammar, for example `bash(git status*)`.
    pub rule: String,
    pub created_at: DateTime<Utc>,
    /// The approval interaction the rule was saved from, when there was one.
    pub source_interaction_id: Option<String>,
}

/// Persistence for [`ToolPermissionRule`] rows.
#[async_trait]
pub trait ToolPermissionRuleStore: Send + Sync {
    /// Save a rule. Saving the same (`user_id`, `effect`, `rule`) again returns
    /// the existing row unchanged.
    async fn upsert(
        &self,
        user_id: &str,
        effect: RuleEffect,
        rule: &str,
        source_interaction_id: Option<&str>,
    ) -> Result<ToolPermissionRule>;

    /// One user's rules, oldest first.
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<ToolPermissionRule>>;

    /// Every user's rules in the space, oldest first.
    async fn list_for_space(&self) -> Result<Vec<ToolPermissionRule>>;

    /// The rule with this id, if it exists.
    async fn get(&self, id: Uuid) -> Result<Option<ToolPermissionRule>>;

    /// Delete a rule. With `Some(user_id)` only that user's row can go, so a
    /// member cannot delete another member's rule. With `None` any row can go
    /// (the admin path). Returns whether a row was deleted.
    async fn delete(&self, id: Uuid, owner: Option<&str>) -> Result<bool>;
}

/// SQLite-backed [`ToolPermissionRuleStore`].
pub struct SqliteToolPermissionRuleStore {
    pool: SqlitePool,
    /// Clock for `created_at`. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl SqliteToolPermissionRuleStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}

/// `SELECT` over every column; sqlx wants a static string, so the tail is
/// concatenated at compile time.
macro_rules! select_rules {
    ($tail:literal) => {
        concat!(
            "SELECT id, user_id, effect, rule, created_at, source_interaction_id \
             FROM tool_permission_rules ",
            $tail
        )
    };
}

fn row_to_rule(row: &sqlx::sqlite::SqliteRow) -> Result<ToolPermissionRule> {
    let id: String = row.get("id");
    let effect: String = row.get("effect");
    let created_at: String = row.get("created_at");
    Ok(ToolPermissionRule {
        id: Uuid::parse_str(&id).context("invalid id in tool_permission_rules")?,
        user_id: row.get("user_id"),
        effect: RuleEffect::parse(&effect)?,
        rule: row.get("rule"),
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .context("invalid created_at in tool_permission_rules")?
            .with_timezone(&Utc),
        source_interaction_id: row.get("source_interaction_id"),
    })
}

#[async_trait]
impl ToolPermissionRuleStore for SqliteToolPermissionRuleStore {
    async fn upsert(
        &self,
        user_id: &str,
        effect: RuleEffect,
        rule: &str,
        source_interaction_id: Option<&str>,
    ) -> Result<ToolPermissionRule> {
        sqlx::query(
            "INSERT INTO tool_permission_rules \
             (id, user_id, effect, rule, created_at, source_interaction_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT (user_id, effect, rule) DO NOTHING",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(effect.as_str())
        .bind(rule)
        .bind(
            self.clock
                .now()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        )
        .bind(source_interaction_id)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query(select_rules!(
            "WHERE user_id = ?1 AND effect = ?2 AND rule = ?3"
        ))
        .bind(user_id)
        .bind(effect.as_str())
        .bind(rule)
        .fetch_one(&self.pool)
        .await?;
        row_to_rule(&row)
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<ToolPermissionRule>> {
        let rows = sqlx::query(select_rules!(
            "WHERE user_id = ?1 ORDER BY created_at ASC, rowid ASC"
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(row_to_rule).collect()
    }

    async fn list_for_space(&self) -> Result<Vec<ToolPermissionRule>> {
        let rows = sqlx::query(select_rules!("ORDER BY created_at ASC, rowid ASC"))
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_rule).collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<ToolPermissionRule>> {
        let row = sqlx::query(select_rules!("WHERE id = ?1"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_rule).transpose()
    }

    async fn delete(&self, id: Uuid, owner: Option<&str>) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM tool_permission_rules \
             WHERE id = ?1 AND (?2 IS NULL OR user_id = ?2)",
        )
        .bind(id.to_string())
        .bind(owner)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// In-memory [`ToolPermissionRuleStore`] for tests.
pub struct InMemoryToolPermissionRuleStore {
    rows: Arc<Mutex<Vec<ToolPermissionRule>>>,
    clock: Arc<dyn Clock>,
}

impl Default for InMemoryToolPermissionRuleStore {
    fn default() -> Self {
        Self {
            rows: Arc::default(),
            clock: Arc::new(SystemClock),
        }
    }
}

impl InMemoryToolPermissionRuleStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<ToolPermissionRule>> {
        match self.rows.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

#[async_trait]
impl ToolPermissionRuleStore for InMemoryToolPermissionRuleStore {
    async fn upsert(
        &self,
        user_id: &str,
        effect: RuleEffect,
        rule: &str,
        source_interaction_id: Option<&str>,
    ) -> Result<ToolPermissionRule> {
        let mut rows = self.lock();
        if let Some(existing) = rows
            .iter()
            .find(|r| r.user_id == user_id && r.effect == effect && r.rule == rule)
        {
            return Ok(existing.clone());
        }
        let saved = ToolPermissionRule {
            id: Uuid::new_v4(),
            user_id: user_id.to_string(),
            effect,
            rule: rule.to_string(),
            created_at: self.clock.now(),
            source_interaction_id: source_interaction_id.map(str::to_string),
        };
        rows.push(saved.clone());
        Ok(saved)
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<ToolPermissionRule>> {
        Ok(self
            .lock()
            .iter()
            .filter(|r| r.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn list_for_space(&self) -> Result<Vec<ToolPermissionRule>> {
        Ok(self.lock().clone())
    }

    async fn get(&self, id: Uuid) -> Result<Option<ToolPermissionRule>> {
        Ok(self.lock().iter().find(|r| r.id == id).cloned())
    }

    async fn delete(&self, id: Uuid, owner: Option<&str>) -> Result<bool> {
        let mut rows = self.lock();
        let before = rows.len();
        rows.retain(|r| !(r.id == id && owner.is_none_or(|o| r.user_id == o)));
        Ok(rows.len() < before)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use assistant_core::clock::FakeClock;

    use super::*;
    use crate::StorageLayer;

    /// Behaviour every store must share.
    async fn assert_store_contract(store: &impl ToolPermissionRuleStore) {
        let first = store
            .upsert(
                "alice",
                RuleEffect::Allow,
                "bash(git status*)",
                Some("int-1"),
            )
            .await
            .unwrap();
        let again = store
            .upsert(
                "alice",
                RuleEffect::Allow,
                "bash(git status*)",
                Some("int-2"),
            )
            .await
            .unwrap();
        assert_eq!(again, first, "upsert is idempotent on (user, effect, rule)");
        assert_eq!(
            again.source_interaction_id.as_deref(),
            Some("int-1"),
            "the first save keeps its source interaction"
        );

        let deny = store
            .upsert("alice", RuleEffect::Deny, "bash(git status*)", None)
            .await
            .unwrap();
        assert_ne!(
            deny.id, first.id,
            "the same rule text with another effect is a new row"
        );
        let bob = store
            .upsert("bob", RuleEffect::Allow, "bash(git status*)", None)
            .await
            .unwrap();
        assert_ne!(
            bob.id, first.id,
            "another user's identical rule is a new row"
        );

        assert_eq!(
            store.list_for_user("alice").await.unwrap(),
            vec![first.clone(), deny.clone()],
            "list_for_user returns only that user's rows, oldest first"
        );
        assert_eq!(
            store.list_for_space().await.unwrap(),
            vec![first.clone(), deny.clone(), bob.clone()],
            "list_for_space returns every user's rows, oldest first"
        );
        assert!(store.list_for_user("nobody").await.unwrap().is_empty());

        assert_eq!(store.get(bob.id).await.unwrap(), Some(bob.clone()));
        assert_eq!(store.get(Uuid::new_v4()).await.unwrap(), None);

        assert!(
            !store.delete(bob.id, Some("alice")).await.unwrap(),
            "an owner-filtered delete cannot remove another user's row"
        );
        assert_eq!(store.get(bob.id).await.unwrap(), Some(bob.clone()));
        assert!(store.delete(bob.id, Some("bob")).await.unwrap());
        assert_eq!(store.get(bob.id).await.unwrap(), None);

        assert!(
            store.delete(first.id, None).await.unwrap(),
            "an unfiltered delete removes any row (the admin path)"
        );
        assert!(
            !store.delete(first.id, None).await.unwrap(),
            "deleting a missing row reports false"
        );
        assert_eq!(store.list_for_space().await.unwrap(), vec![deny]);
    }

    #[tokio::test]
    async fn sqlite_store_satisfies_the_contract() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        assert_store_contract(&storage.tool_permission_rule_store()).await;
    }

    #[tokio::test]
    async fn in_memory_store_satisfies_the_contract() {
        assert_store_contract(&InMemoryToolPermissionRuleStore::new()).await;
    }

    #[tokio::test]
    async fn stores_stamp_created_at_from_the_injected_clock() {
        let seed = Utc.with_ymd_and_hms(2026, 11, 25, 12, 0, 0).unwrap();
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let sqlite = SqliteToolPermissionRuleStore::new(storage.pool.clone())
            .with_clock(Arc::new(FakeClock::new(seed)));
        let memory =
            InMemoryToolPermissionRuleStore::new().with_clock(Arc::new(FakeClock::new(seed)));

        let from_sqlite = sqlite
            .upsert("alice", RuleEffect::Allow, "bash(ls*)", None)
            .await
            .unwrap();
        let from_memory = memory
            .upsert("alice", RuleEffect::Allow, "bash(ls*)", None)
            .await
            .unwrap();
        assert_eq!(from_sqlite.created_at, seed);
        assert_eq!(from_memory.created_at, seed);
    }

    #[tokio::test]
    async fn sqlite_store_round_trips_every_field() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.tool_permission_rule_store();
        let saved = store
            .upsert("alice", RuleEffect::Deny, "bash(rm *)", Some("int-9"))
            .await
            .unwrap();

        let loaded = store.get(saved.id).await.unwrap().unwrap();
        assert_eq!(loaded, saved);
        assert_eq!(loaded.effect, RuleEffect::Deny);
        assert_eq!(loaded.source_interaction_id.as_deref(), Some("int-9"));
    }

    #[tokio::test]
    async fn sqlite_rejects_an_unknown_effect() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let result = sqlx::query(
            "INSERT INTO tool_permission_rules (id, user_id, effect, rule, created_at) \
             VALUES ('x', 'alice', 'maybe', 'bash(*)', '2026-11-25T00:00:00.000Z')",
        )
        .execute(&storage.pool)
        .await;
        assert!(
            result.is_err(),
            "the effect column only admits allow or deny"
        );
    }
}
