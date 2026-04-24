//! SQLite-backed [`ApiKeyStore`] implementation.

use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use assistant_auth::api_keys::{ApiKeyRecord, ApiKeyStore};
use assistant_core::identity::{OrgId, Role, Scope, SpaceId, UserId};

/// SQLite-backed API key store.
pub struct SqliteApiKeyStore {
    pool: SqlitePool,
}

impl SqliteApiKeyStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn role_from_str(s: &str) -> Role {
    match s {
        "org_admin" | "OrgAdmin" => Role::OrgAdmin,
        "space_admin" | "SpaceAdmin" => Role::SpaceAdmin,
        "member" | "Member" => Role::Member,
        _ => Role::Viewer,
    }
}

fn role_to_str(role: &Role) -> &'static str {
    match role {
        Role::OrgAdmin => "org_admin",
        Role::SpaceAdmin => "space_admin",
        Role::Member => "member",
        Role::Viewer => "viewer",
    }
}

fn serialize_space_roles(roles: &HashMap<SpaceId, Role>) -> String {
    let map: HashMap<&str, &str> = roles
        .iter()
        .map(|(k, v)| (k.0.as_str(), role_to_str(v)))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".into())
}

fn deserialize_space_roles(json: &str) -> HashMap<SpaceId, Role> {
    let map: HashMap<String, String> = serde_json::from_str(json).unwrap_or_default();
    map.into_iter()
        .map(|(k, v)| (SpaceId::from(k), role_from_str(&v)))
        .collect()
}

fn serialize_scopes(scopes: &[Scope]) -> String {
    let entries: Vec<serde_json::Value> = scopes
        .iter()
        .map(|s| {
            serde_json::json!({
                "resource": s.resource.to_string(),
                "action": s.action.to_string(),
            })
        })
        .collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into())
}

fn deserialize_scopes(json: &str) -> Vec<Scope> {
    let entries: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    entries
        .into_iter()
        .filter_map(|v| {
            let resource = v.get("resource")?.as_str()?.parse().ok()?;
            let action = v.get("action")?.as_str()?.parse().ok()?;
            Some(Scope::new(resource, action))
        })
        .collect()
}

fn row_to_record(row: sqlx::sqlite::SqliteRow) -> ApiKeyRecord {
    let space_roles_json: String = row.get("space_roles");
    let scopes_json: String = row.get("scopes");
    let expires_at: Option<DateTime<Utc>> = row.get("expires_at");

    ApiKeyRecord {
        id: row.get("id"),
        user_id: UserId(row.get("user_id")),
        org_id: OrgId(row.get("org_id")),
        name: row.get("name"),
        key_hash: row.get("key_hash"),
        key_prefix: row.get("key_prefix"),
        space_roles: deserialize_space_roles(&space_roles_json),
        scopes: deserialize_scopes(&scopes_json),
        expires_at,
        created_at: row.get("created_at"),
    }
}

#[async_trait]
impl ApiKeyStore for SqliteApiKeyStore {
    async fn store_key(&self, record: ApiKeyRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO api_keys (id, user_id, org_id, name, key_hash, key_prefix, space_roles, scopes, expires_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(&record.user_id.0)
        .bind(&record.org_id.0)
        .bind(&record.name)
        .bind(&record.key_hash)
        .bind(&record.key_prefix)
        .bind(serialize_space_roles(&record.space_roles))
        .bind(serialize_scopes(&record.scopes))
        .bind(record.expires_at)
        .bind(record.created_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("storing API key: {}", record.id))?;
        Ok(())
    }

    async fn get_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyRecord>> {
        let row = sqlx::query(
            "SELECT id, user_id, org_id, name, key_hash, key_prefix, space_roles, scopes, expires_at, created_at
             FROM api_keys WHERE key_hash = ?",
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_record))
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ApiKeyRecord>> {
        let rows = sqlx::query(
            "SELECT id, user_id, org_id, name, key_hash, key_prefix, space_roles, scopes, expires_at, created_at
             FROM api_keys WHERE user_id = ? ORDER BY created_at",
        )
        .bind(&user_id.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_record).collect())
    }

    async fn revoke(&self, key_id: &str, user_id: &UserId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM api_keys WHERE id = ? AND user_id = ?")
            .bind(key_id)
            .bind(&user_id.0)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
