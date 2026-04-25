use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

/// Default output destination for scheduler-originated turns (cron tasks and
/// heartbeats).  Operator-configured; the agent is unaware of routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeChannel {
    /// Interface name matching the adapter's `name()` (e.g. `"slack"`, `"signal"`, `"matrix"`).
    pub home_interface: String,
    /// Platform-native channel address (e.g. `"#ops"`, `"+12345678901"`, `"!room:server"`).
    pub home_channel: String,
}

#[derive(Debug, Clone)]
pub struct PersonaRecord {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    /// Skill access mode: "all", "whitelist", or "blacklist". Defaults to "all".
    pub skill_access_mode: String,
    /// Per-persona turn timeout in seconds. `None` means use the compiled-in
    /// default (10 800 s / 3 h).
    pub turn_timeout_secs: Option<u64>,
    /// Default output destination for scheduler-originated turns. `None` means
    /// scheduler output is stored in conversation history only (no delivery).
    pub home_channel: Option<HomeChannel>,
    /// The user who owns this persona (multi-user scoping).
    /// `None` means org-owned (visible to all org members).
    pub owner_user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PersonaStore {
    pool: SqlitePool,
}

impl PersonaStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn ensure_default(&self) -> Result<PersonaRecord> {
        sqlx::query(
            "INSERT INTO personas (id, name, is_default) VALUES ('default', 'Default', 1) \
             ON CONFLICT(id) DO NOTHING",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE personas
             SET is_default = CASE WHEN id = 'default' THEN 1 ELSE 0 END,
                 updated_at = CURRENT_TIMESTAMP
             WHERE NOT EXISTS (
                 SELECT 1 FROM personas WHERE is_default = 1
             )",
        )
        .execute(&self.pool)
        .await?;

        self.get("default")
            .await?
            .ok_or_else(|| anyhow::anyhow!("default persona missing after ensure_default"))
    }

    pub async fn ensure_exists(&self, id: &str) -> Result<PersonaRecord> {
        if let Some(existing) = self.get(id).await? {
            return Ok(existing);
        }

        let inferred_name = id.replace(['_', '-'], " ");
        sqlx::query(
            "INSERT INTO personas (id, name, is_default) VALUES (?1, ?2, 0)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(id)
        .bind(if inferred_name.is_empty() {
            id.to_string()
        } else {
            inferred_name
        })
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("persona '{}' missing after ensure_exists", id))
    }

    pub async fn list(&self) -> Result<Vec<PersonaRecord>> {
        let rows = sqlx::query(
            "SELECT id, name, is_default, skill_access_mode, turn_timeout_secs, home_interface, home_channel, owner_user_id, created_at, updated_at
             FROM personas
             ORDER BY is_default DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_record).collect()
    }

    pub async fn set_default(&self, id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM personas WHERE id = ?1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        if exists == 0 {
            anyhow::bail!("persona '{}' does not exist", id);
        }

        sqlx::query("UPDATE personas SET is_default = 0, updated_at = CURRENT_TIMESTAMP")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE personas
             SET is_default = 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn default_id(&self) -> Result<String> {
        let id = sqlx::query_scalar::<_, String>(
            "SELECT id
             FROM personas
             WHERE is_default = 1
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_else(|| "default".to_string());
        Ok(id)
    }

    pub async fn get(&self, id: &str) -> Result<Option<PersonaRecord>> {
        let row = sqlx::query(
            "SELECT id, name, is_default, skill_access_mode, turn_timeout_secs, home_interface, home_channel, owner_user_id, created_at, updated_at
             FROM personas
             WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_record).transpose()
    }

    /// Fetch a persona only if the caller is allowed to see it.
    ///
    /// Returns the persona when it is org-owned (`owner_user_id IS NULL`)
    /// **or** owned by the given `user_id`. Returns `Ok(None)` otherwise.
    pub async fn get_accessible(&self, id: &str, user_id: &str) -> Result<Option<PersonaRecord>> {
        anyhow::ensure!(!user_id.trim().is_empty(), "user_id must be non-empty");

        let row = sqlx::query(
            "SELECT id, name, is_default, skill_access_mode, turn_timeout_secs, home_interface, home_channel, owner_user_id, created_at, updated_at
             FROM personas
             WHERE id = ?1 AND (owner_user_id IS NULL OR owner_user_id = ?2)",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_record).transpose()
    }

    pub async fn create(&self, id: &str, name: &str) -> Result<PersonaRecord> {
        sqlx::query("INSERT INTO personas (id, name, is_default) VALUES (?1, ?2, 0)")
            .bind(id)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    anyhow::anyhow!(
                        "persona with id '{}' already exists (UNIQUE constraint)",
                        id
                    )
                } else {
                    anyhow::anyhow!("failed to create persona '{}': {}", id, e)
                }
            })?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("persona '{}' missing after create", id))
    }

    /// Set a per-persona turn timeout. `secs` must be > 0.
    pub async fn set_turn_timeout(&self, id: &str, secs: u64) -> Result<()> {
        anyhow::ensure!(secs > 0, "turn_timeout_secs must be greater than 0");
        let rows = sqlx::query(
            "UPDATE personas
             SET turn_timeout_secs = ?1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?2",
        )
        .bind(secs as i64)
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        anyhow::ensure!(rows > 0, "persona '{}' not found", id);
        Ok(())
    }

    /// Set the home channel for scheduler-originated output routing.
    /// Both `interface` and `channel` must be non-empty.
    pub async fn set_home_channel(
        &self,
        id: &str,
        home_interface: &str,
        home_channel: &str,
    ) -> Result<()> {
        anyhow::ensure!(
            !home_interface.is_empty() && !home_channel.is_empty(),
            "home_interface and home_channel must both be non-empty"
        );
        let rows = sqlx::query(
            "UPDATE personas
             SET home_interface = ?1, home_channel = ?2, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?3",
        )
        .bind(home_interface)
        .bind(home_channel)
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        anyhow::ensure!(rows > 0, "persona '{}' not found", id);
        Ok(())
    }

    /// Clear the home channel, disabling scheduler output routing for this persona.
    pub async fn clear_home_channel(&self, id: &str) -> Result<()> {
        let rows = sqlx::query(
            "UPDATE personas
             SET home_interface = NULL, home_channel = NULL, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        anyhow::ensure!(rows > 0, "persona '{}' not found", id);
        Ok(())
    }

    /// List personas accessible to a given user: org-owned (`owner_user_id IS NULL`)
    /// plus those owned by the user (`owner_user_id = ?`).
    pub async fn list_accessible(&self, user_id: &str) -> Result<Vec<PersonaRecord>> {
        anyhow::ensure!(!user_id.trim().is_empty(), "user_id must be non-empty");

        let rows = sqlx::query(
            "SELECT id, name, is_default, skill_access_mode, turn_timeout_secs, home_interface, home_channel, owner_user_id, created_at, updated_at
             FROM personas
             WHERE owner_user_id IS NULL OR owner_user_id = ?1
             ORDER BY is_default DESC, id ASC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_record).collect()
    }

    /// Create a user-owned persona.
    pub async fn create_owned(
        &self,
        id: &str,
        name: &str,
        owner_user_id: &str,
    ) -> Result<PersonaRecord> {
        anyhow::ensure!(
            !owner_user_id.trim().is_empty(),
            "owner_user_id must be non-empty"
        );

        sqlx::query(
            "INSERT INTO personas (id, name, is_default, owner_user_id) VALUES (?1, ?2, 0, ?3)",
        )
        .bind(id)
        .bind(name)
        .bind(owner_user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                anyhow::anyhow!(
                    "persona with id '{}' already exists (UNIQUE constraint)",
                    id
                )
            } else {
                anyhow::anyhow!("failed to create persona '{}': {}", id, e)
            }
        })?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("persona '{}' missing after create_owned", id))
    }

    /// Clear the per-persona turn timeout, reverting to the compiled-in default.
    pub async fn clear_turn_timeout(&self, id: &str) -> Result<()> {
        let rows = sqlx::query(
            "UPDATE personas
             SET turn_timeout_secs = NULL, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        anyhow::ensure!(rows > 0, "persona '{}' not found", id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::StorageLayer;

    #[tokio::test]
    async fn create_returns_error_on_duplicate_id() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.create("foo", "Foo").await.unwrap();
        let result = store.create("foo", "Foo Again").await;
        assert!(result.is_err(), "expected Err on duplicate id, got Ok");
    }

    #[tokio::test]
    async fn turn_timeout_defaults_to_none() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.ensure_default().await.unwrap();
        let persona = store.get("default").await.unwrap().unwrap();
        assert!(
            persona.turn_timeout_secs.is_none(),
            "new persona should have no explicit timeout"
        );
    }

    #[tokio::test]
    async fn set_and_clear_turn_timeout() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.create("bot", "Bot").await.unwrap();

        store.set_turn_timeout("bot", 7200).await.unwrap();
        let persona = store.get("bot").await.unwrap().unwrap();
        assert_eq!(
            persona.turn_timeout_secs,
            Some(7200),
            "turn_timeout_secs should reflect set value"
        );

        store.clear_turn_timeout("bot").await.unwrap();
        let persona = store.get("bot").await.unwrap().unwrap();
        assert!(
            persona.turn_timeout_secs.is_none(),
            "turn_timeout_secs should be None after clear"
        );
    }

    #[tokio::test]
    async fn set_turn_timeout_rejects_zero() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.create("bot", "Bot").await.unwrap();
        let result = store.set_turn_timeout("bot", 0).await;
        assert!(result.is_err(), "zero timeout should be rejected");
    }

    #[tokio::test]
    async fn set_turn_timeout_unknown_persona_returns_error() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        let result = store.set_turn_timeout("nonexistent", 3600).await;
        assert!(
            result.is_err(),
            "setting timeout on unknown persona should error"
        );
    }

    #[tokio::test]
    async fn home_channel_defaults_to_none() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.ensure_default().await.unwrap();
        let persona = store.get("default").await.unwrap().unwrap();
        assert!(
            persona.home_channel.is_none(),
            "new persona should have no home channel"
        );
    }

    #[tokio::test]
    async fn set_and_clear_home_channel() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.create("bot", "Bot").await.unwrap();

        store
            .set_home_channel("bot", "slack", "#ops")
            .await
            .unwrap();
        let persona = store.get("bot").await.unwrap().unwrap();
        let hc = persona.home_channel.as_ref().unwrap();
        assert_eq!(hc.home_interface, "slack", "interface should be slack");
        assert_eq!(hc.home_channel, "#ops", "channel should be #ops");

        store.clear_home_channel("bot").await.unwrap();
        let persona = store.get("bot").await.unwrap().unwrap();
        assert!(
            persona.home_channel.is_none(),
            "home_channel should be None after clear"
        );
    }

    #[tokio::test]
    async fn set_home_channel_rejects_empty_fields() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.create("bot", "Bot").await.unwrap();

        assert!(
            store.set_home_channel("bot", "", "#ops").await.is_err(),
            "empty interface should be rejected"
        );
        assert!(
            store.set_home_channel("bot", "slack", "").await.is_err(),
            "empty channel should be rejected"
        );
    }

    #[tokio::test]
    async fn set_home_channel_unknown_persona_returns_error() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        let result = store.set_home_channel("nonexistent", "slack", "#ops").await;
        assert!(
            result.is_err(),
            "setting home channel on unknown persona should error"
        );
    }

    #[tokio::test]
    async fn home_channel_visible_in_list() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.ensure_default().await.unwrap();
        store.create("notifier", "Notifier").await.unwrap();
        store
            .set_home_channel("notifier", "signal", "+12345678901")
            .await
            .unwrap();

        let list = store.list().await.unwrap();
        let notifier = list.iter().find(|p| p.id == "notifier").unwrap();
        let hc = notifier.home_channel.as_ref().unwrap();
        assert_eq!(hc.home_interface, "signal");
        assert_eq!(hc.home_channel, "+12345678901");

        let default = list.iter().find(|p| p.id == "default").unwrap();
        assert!(default.home_channel.is_none());
    }

    #[tokio::test]
    async fn turn_timeout_visible_in_list() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.ensure_default().await.unwrap();
        store.create("slow", "Slow").await.unwrap();
        store.set_turn_timeout("slow", 21600).await.unwrap();

        let list = store.list().await.unwrap();
        let slow = list.iter().find(|p| p.id == "slow").unwrap();
        assert_eq!(slow.turn_timeout_secs, Some(21600));

        let default = list.iter().find(|p| p.id == "default").unwrap();
        assert!(default.turn_timeout_secs.is_none());
    }

    // -----------------------------------------------------------------------
    // Owner / user-scoping tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn create_owned_sets_owner_user_id() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        let persona = store
            .create_owned("alice-bot", "Alice Bot", "alice")
            .await
            .unwrap();
        assert_eq!(
            persona.owner_user_id.as_deref(),
            Some("alice"),
            "owned persona should persist owner_user_id"
        );
    }

    #[tokio::test]
    async fn org_owned_persona_has_null_owner() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        let persona = store.create("shared-bot", "Shared Bot").await.unwrap();
        assert!(
            persona.owner_user_id.is_none(),
            "org-owned persona should have no owner"
        );
    }

    #[tokio::test]
    async fn list_accessible_includes_org_owned_and_user_owned() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        // Create an org-owned persona (NULL owner)
        store.create("shared", "Shared").await.unwrap();
        // Create user-owned personas
        store
            .create_owned("alice-bot", "Alice Bot", "alice")
            .await
            .unwrap();
        store
            .create_owned("bob-bot", "Bob Bot", "bob")
            .await
            .unwrap();

        // Alice should see shared + alice-bot, but not bob-bot
        let alice_list = store.list_accessible("alice").await.unwrap();
        let alice_ids: Vec<&str> = alice_list.iter().map(|p| p.id.as_str()).collect();
        assert!(alice_ids.contains(&"shared"), "Alice should see org-owned");
        assert!(alice_ids.contains(&"alice-bot"), "Alice should see her own");
        assert!(
            !alice_ids.contains(&"bob-bot"),
            "Alice should not see Bob's"
        );

        // Bob should see shared + bob-bot, but not alice-bot
        let bob_list = store.list_accessible("bob").await.unwrap();
        let bob_ids: Vec<&str> = bob_list.iter().map(|p| p.id.as_str()).collect();
        assert!(bob_ids.contains(&"shared"), "Bob should see org-owned");
        assert!(bob_ids.contains(&"bob-bot"), "Bob should see his own");
        assert!(
            !bob_ids.contains(&"alice-bot"),
            "Bob should not see Alice's"
        );
    }

    #[tokio::test]
    async fn list_returns_all_personas_regardless_of_owner() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.create("shared", "Shared").await.unwrap();
        store
            .create_owned("alice-bot", "Alice Bot", "alice")
            .await
            .unwrap();

        let all = store.list().await.unwrap();
        // 3 = seeded "default" from migration + "shared" + "alice-bot"
        assert_eq!(all.len(), 3, "list() should return all personas");
        let ids: Vec<&str> = all.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"shared"), "should include shared persona");
        assert!(
            ids.contains(&"alice-bot"),
            "should include alice-bot persona"
        );
    }

    #[tokio::test]
    async fn list_accessible_rejects_empty_user_id() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        let result = store.list_accessible("").await;
        assert!(result.is_err(), "empty user_id should be rejected");
    }

    #[tokio::test]
    async fn create_owned_rejects_empty_owner() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        let result = store.create_owned("bot", "Bot", "").await;
        assert!(result.is_err(), "empty owner_user_id should be rejected");
    }

    #[tokio::test]
    async fn get_accessible_returns_org_owned() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.create("shared", "Shared").await.unwrap();

        let result = store.get_accessible("shared", "alice").await.unwrap();
        assert!(
            result.is_some(),
            "org-owned personas should be accessible to any user"
        );
    }

    #[tokio::test]
    async fn get_accessible_returns_own_persona() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store
            .create_owned("alice-bot", "Alice Bot", "alice")
            .await
            .unwrap();

        let result = store.get_accessible("alice-bot", "alice").await.unwrap();
        assert!(result.is_some(), "user should see their own persona");
    }

    #[tokio::test]
    async fn get_accessible_denies_other_user() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store
            .create_owned("alice-bot", "Alice Bot", "alice")
            .await
            .unwrap();

        let result = store.get_accessible("alice-bot", "bob").await.unwrap();
        assert!(
            result.is_none(),
            "user should not see another user's persona"
        );
    }

    #[tokio::test]
    async fn get_accessible_rejects_empty_user_id() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        let result = store.get_accessible("any", "").await;
        assert!(result.is_err(), "empty user_id should be rejected");
    }
}

fn row_to_record(row: sqlx::sqlite::SqliteRow) -> Result<PersonaRecord> {
    let home_interface: Option<String> = row.try_get("home_interface").unwrap_or(None);
    let home_channel_val: Option<String> = row.try_get("home_channel").unwrap_or(None);
    let home_channel = match (home_interface, home_channel_val) {
        (Some(iface), Some(chan)) if !iface.is_empty() && !chan.is_empty() => Some(HomeChannel {
            home_interface: iface,
            home_channel: chan,
        }),
        _ => None,
    };

    Ok(PersonaRecord {
        id: row.get("id"),
        name: row.get("name"),
        is_default: row.get::<i64, _>("is_default") != 0,
        skill_access_mode: row
            .try_get("skill_access_mode")
            .unwrap_or_else(|_| "all".to_string()),
        turn_timeout_secs: row
            .try_get::<Option<i64>, _>("turn_timeout_secs")
            .unwrap_or(None)
            .map(|v| v as u64),
        home_channel,
        owner_user_id: row.try_get("owner_user_id").unwrap_or(None),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
