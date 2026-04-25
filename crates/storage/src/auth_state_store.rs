//! SQLite-backed implementations of the auth crate's store traits.
//!
//! Bridges [`ClientStore`], [`AuthCodeStore`], [`RefreshTokenStore`], and
//! [`DeviceCodeStore`] to the `oauth_clients`, `auth_codes`, `refresh_tokens`,
//! and `device_codes` tables in org.db.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_auth::oauth2::clients::ClientStore;
use assistant_auth::oauth2::device::{DeviceCodeStore, DeviceState};
use assistant_auth::oauth2::server::{
    AuthCode, AuthCodeStore, RefreshTokenStore, StoredRefreshToken,
};
use assistant_core::auth::ClientInfo;

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

fn to_json(v: &[String]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".into())
}

fn from_json(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// SqliteClientStore
// ---------------------------------------------------------------------------

/// SQLite-backed OAuth2 client store.
pub struct SqliteClientStore {
    pool: SqlitePool,
}

impl SqliteClientStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ClientStore for SqliteClientStore {
    async fn register(&self, info: ClientInfo) -> Result<()> {
        sqlx::query(
            "INSERT INTO oauth_clients (client_id, client_name, client_secret_hash, redirect_uris_json, grant_types_json, token_endpoint_auth_method)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&info.client_id)
        .bind(&info.client_name)
        .bind(&info.client_secret)
        .bind(to_json(&info.redirect_uris))
        .bind(to_json(&info.grant_types))
        .bind(&info.token_endpoint_auth_method)
        .execute(&self.pool)
        .await
        .with_context(|| format!("registering OAuth client: {}", info.client_id))?;
        Ok(())
    }

    async fn get(&self, client_id: &str) -> Result<Option<ClientInfo>> {
        let row = sqlx::query(
            "SELECT client_id, client_name, client_secret_hash, redirect_uris_json, grant_types_json, token_endpoint_auth_method
             FROM oauth_clients WHERE client_id = ?",
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ClientInfo {
            client_id: r.get("client_id"),
            client_name: r.get("client_name"),
            client_secret: r.get("client_secret_hash"),
            redirect_uris: from_json(r.get("redirect_uris_json")),
            grant_types: from_json(r.get("grant_types_json")),
            token_endpoint_auth_method: r.get("token_endpoint_auth_method"),
        }))
    }

    async fn list(&self) -> Result<Vec<ClientInfo>> {
        let rows = sqlx::query(
            "SELECT client_id, client_name, client_secret_hash, redirect_uris_json, grant_types_json, token_endpoint_auth_method
             FROM oauth_clients ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ClientInfo {
                client_id: r.get("client_id"),
                client_name: r.get("client_name"),
                client_secret: r.get("client_secret_hash"),
                redirect_uris: from_json(r.get("redirect_uris_json")),
                grant_types: from_json(r.get("grant_types_json")),
                token_endpoint_auth_method: r.get("token_endpoint_auth_method"),
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// SqliteAuthCodeStore
// ---------------------------------------------------------------------------

/// SQLite-backed authorization code store.
pub struct SqliteAuthCodeStore {
    pool: SqlitePool,
}

impl SqliteAuthCodeStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuthCodeStore for SqliteAuthCodeStore {
    async fn store_code(&self, code: AuthCode) -> Result<()> {
        sqlx::query(
            "INSERT INTO auth_codes (code_hash, user_id, client_id, redirect_uri, pkce_challenge, scopes_json, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&code.code)
        .bind(&code.user_id)
        .bind(&code.client_id)
        .bind(&code.redirect_uri)
        .bind(&code.pkce_challenge)
        .bind(to_json(&code.scopes))
        .bind(code.expires_at)
        .execute(&self.pool)
        .await
        .with_context(|| "storing auth code")?;
        Ok(())
    }

    async fn take_code(&self, code: &str) -> Result<Option<AuthCode>> {
        // Single-use: fetch then delete in a transaction-like sequence.
        let row = sqlx::query(
            "SELECT code_hash, user_id, client_id, redirect_uri, pkce_challenge, scopes_json, expires_at
             FROM auth_codes WHERE code_hash = ?",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        // Delete immediately (single-use).
        sqlx::query("DELETE FROM auth_codes WHERE code_hash = ?")
            .bind(code)
            .execute(&self.pool)
            .await?;

        Ok(Some(AuthCode {
            code: row.get("code_hash"),
            user_id: row.get("user_id"),
            client_id: row.get("client_id"),
            redirect_uri: row.get("redirect_uri"),
            pkce_challenge: row.get("pkce_challenge"),
            scopes: from_json(row.get("scopes_json")),
            expires_at: row.get("expires_at"),
        }))
    }
}

// ---------------------------------------------------------------------------
// SqliteRefreshTokenStore
// ---------------------------------------------------------------------------

/// SQLite-backed refresh token store.
pub struct SqliteRefreshTokenStore {
    pool: SqlitePool,
}

impl SqliteRefreshTokenStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefreshTokenStore for SqliteRefreshTokenStore {
    async fn store_token(&self, token: StoredRefreshToken) -> Result<()> {
        sqlx::query(
            "INSERT INTO refresh_tokens (token_hash, user_id, client_id, scopes_json, expires_at, consumed)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&token.token)
        .bind(&token.user_id)
        .bind(&token.client_id)
        .bind(to_json(&token.scopes))
        .bind(token.expires_at)
        .bind(token.consumed as i32)
        .execute(&self.pool)
        .await
        .with_context(|| "storing refresh token")?;
        Ok(())
    }

    async fn get_token(&self, token: &str) -> Result<Option<StoredRefreshToken>> {
        let row = sqlx::query(
            "SELECT token_hash, user_id, client_id, scopes_json, expires_at, consumed
             FROM refresh_tokens WHERE token_hash = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let consumed: i32 = r.get("consumed");
            StoredRefreshToken {
                token: r.get("token_hash"),
                user_id: r.get("user_id"),
                client_id: r.get("client_id"),
                scopes: from_json(r.get("scopes_json")),
                expires_at: r.get("expires_at"),
                consumed: consumed != 0,
            }
        }))
    }

    async fn consume_token(&self, token: &str) -> Result<()> {
        sqlx::query("UPDATE refresh_tokens SET consumed = 1 WHERE token_hash = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn revoke_all(&self, user_id: &str, client_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ? AND client_id = ?")
            .bind(user_id)
            .bind(client_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn revoke_for_user_except(&self, user_id: &str, except_token: &str) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ? AND token_hash != ?")
                .bind(user_id)
                .bind(except_token)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }
}

// ---------------------------------------------------------------------------
// SqliteDeviceCodeStore
// ---------------------------------------------------------------------------

/// SQLite-backed device code store.
pub struct SqliteDeviceCodeStore {
    pool: SqlitePool,
}

impl SqliteDeviceCodeStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeviceCodeStore for SqliteDeviceCodeStore {
    async fn store(&self, device_code: &str, state: DeviceState) -> Result<()> {
        sqlx::query(
            "INSERT INTO device_codes (device_code_hash, user_code, client_id, scopes_json, authorized_user, expires_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(device_code)
        .bind(&state.user_code)
        .bind(&state.client_id)
        .bind(to_json(&state.scopes))
        .bind(&state.authorized_user)
        .bind(state.expires_at)
        .execute(&self.pool)
        .await
        .with_context(|| "storing device code")?;
        Ok(())
    }

    async fn get_by_device_code(&self, device_code: &str) -> Result<Option<DeviceState>> {
        let row = sqlx::query(
            "SELECT device_code_hash, user_code, client_id, scopes_json, authorized_user, expires_at
             FROM device_codes WHERE device_code_hash = ?",
        )
        .bind(device_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_device_state))
    }

    async fn get_by_user_code(&self, user_code: &str) -> Result<Option<DeviceState>> {
        let row = sqlx::query(
            "SELECT device_code_hash, user_code, client_id, scopes_json, authorized_user, expires_at
             FROM device_codes WHERE user_code = ?",
        )
        .bind(user_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_device_state))
    }

    async fn update(&self, device_code: &str, state: DeviceState) -> Result<()> {
        sqlx::query(
            "UPDATE device_codes SET user_code = ?, client_id = ?, scopes_json = ?, authorized_user = ?, expires_at = ?
             WHERE device_code_hash = ?",
        )
        .bind(&state.user_code)
        .bind(&state.client_id)
        .bind(to_json(&state.scopes))
        .bind(&state.authorized_user)
        .bind(state.expires_at)
        .bind(device_code)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove(&self, device_code: &str) -> Result<()> {
        sqlx::query("DELETE FROM device_codes WHERE device_code_hash = ?")
            .bind(device_code)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn row_to_device_state(r: sqlx::sqlite::SqliteRow) -> DeviceState {
    DeviceState {
        device_code: r.get("device_code_hash"),
        user_code: r.get("user_code"),
        client_id: r.get("client_id"),
        scopes: from_json(r.get("scopes_json")),
        expires_at: r.get("expires_at"),
        authorized_user: r.get("authorized_user"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_storage::OrgStorageLayer;
    use chrono::{Duration, Utc};

    async fn setup() -> OrgStorageLayer {
        OrgStorageLayer::new_in_memory().await.unwrap()
    }

    // -- ClientStore --

    #[tokio::test]
    async fn client_register_and_get() {
        let layer = setup().await;
        let store = layer.client_store();

        let info = ClientInfo {
            client_id: "cli_app".into(),
            client_name: "CLI App".into(),
            client_secret: None,
            redirect_uris: vec!["http://localhost:8080/callback".into()],
            grant_types: vec!["authorization_code".into()],
            token_endpoint_auth_method: "none".into(),
        };
        store.register(info).await.unwrap();

        let found = store.get("cli_app").await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.client_name, "CLI App");
        assert_eq!(found.redirect_uris, vec!["http://localhost:8080/callback"]);
    }

    #[tokio::test]
    async fn client_list() {
        let layer = setup().await;
        let store = layer.client_store();

        store
            .register(ClientInfo {
                client_id: "app_1".into(),
                client_name: "App 1".into(),
                client_secret: None,
                redirect_uris: vec![],
                grant_types: vec![],
                token_endpoint_auth_method: "none".into(),
            })
            .await
            .unwrap();
        store
            .register(ClientInfo {
                client_id: "app_2".into(),
                client_name: "App 2".into(),
                client_secret: Some("secret".into()),
                redirect_uris: vec![],
                grant_types: vec![],
                token_endpoint_auth_method: "client_secret_post".into(),
            })
            .await
            .unwrap();

        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn client_get_nonexistent() {
        let layer = setup().await;
        let store = layer.client_store();
        let found = store.get("no_such").await.unwrap();
        assert!(found.is_none());
    }

    // -- AuthCodeStore --

    #[tokio::test]
    async fn auth_code_store_and_take() {
        let layer = setup().await;
        let store = layer.auth_code_store();

        let code = AuthCode {
            code: "abc123".into(),
            user_id: "usr_1".into(),
            client_id: "cli_app".into(),
            redirect_uri: "http://localhost/callback".into(),
            scopes: vec!["read".into(), "write".into()],
            pkce_challenge: Some("challenge_value".into()),
            expires_at: Utc::now() + Duration::minutes(5),
        };
        store.store_code(code).await.unwrap();

        // First take succeeds.
        let taken = store.take_code("abc123").await.unwrap();
        assert!(taken.is_some());
        let taken = taken.unwrap();
        assert_eq!(taken.user_id, "usr_1");
        assert_eq!(taken.scopes, vec!["read", "write"]);

        // Second take returns None (single-use).
        let again = store.take_code("abc123").await.unwrap();
        assert!(again.is_none());
    }

    #[tokio::test]
    async fn auth_code_take_nonexistent() {
        let layer = setup().await;
        let store = layer.auth_code_store();
        let result = store.take_code("no_such").await.unwrap();
        assert!(result.is_none());
    }

    // -- RefreshTokenStore --

    #[tokio::test]
    async fn refresh_token_store_and_get() {
        let layer = setup().await;
        let store = layer.refresh_token_store();

        let token = StoredRefreshToken {
            token: "refresh_abc".into(),
            user_id: "usr_1".into(),
            client_id: "cli_app".into(),
            scopes: vec!["read".into()],
            expires_at: Utc::now() + Duration::days(30),
            consumed: false,
        };
        store.store_token(token).await.unwrap();

        let found = store.get_token("refresh_abc").await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert!(!found.consumed);
        assert_eq!(found.user_id, "usr_1");
    }

    #[tokio::test]
    async fn refresh_token_consume() {
        let layer = setup().await;
        let store = layer.refresh_token_store();

        store
            .store_token(StoredRefreshToken {
                token: "refresh_abc".into(),
                user_id: "usr_1".into(),
                client_id: "cli_app".into(),
                scopes: vec![],
                expires_at: Utc::now() + Duration::days(30),
                consumed: false,
            })
            .await
            .unwrap();

        store.consume_token("refresh_abc").await.unwrap();

        let found = store.get_token("refresh_abc").await.unwrap().unwrap();
        assert!(found.consumed);
    }

    #[tokio::test]
    async fn refresh_token_revoke_all() {
        let layer = setup().await;
        let store = layer.refresh_token_store();

        for i in 0..3 {
            store
                .store_token(StoredRefreshToken {
                    token: format!("tok_{i}"),
                    user_id: "usr_1".into(),
                    client_id: "cli_app".into(),
                    scopes: vec![],
                    expires_at: Utc::now() + Duration::days(30),
                    consumed: false,
                })
                .await
                .unwrap();
        }

        store.revoke_all("usr_1", "cli_app").await.unwrap();

        for i in 0..3 {
            let found = store.get_token(&format!("tok_{i}")).await.unwrap();
            assert!(found.is_none(), "token tok_{i} should be revoked");
        }
    }

    #[tokio::test]
    async fn refresh_token_revoke_for_user_except() {
        let layer = setup().await;
        let store = layer.refresh_token_store();

        // Three sessions for usr_1, one for usr_2.
        for i in 0..3 {
            store
                .store_token(StoredRefreshToken {
                    token: format!("alice_tok_{i}"),
                    user_id: "usr_1".into(),
                    client_id: format!("cli_{i}"),
                    scopes: vec![],
                    expires_at: Utc::now() + Duration::days(30),
                    consumed: false,
                })
                .await
                .unwrap();
        }
        store
            .store_token(StoredRefreshToken {
                token: "bob_tok".into(),
                user_id: "usr_2".into(),
                client_id: "cli_x".into(),
                scopes: vec![],
                expires_at: Utc::now() + Duration::days(30),
                consumed: false,
            })
            .await
            .unwrap();

        let n = store
            .revoke_for_user_except("usr_1", "alice_tok_1")
            .await
            .unwrap();
        assert_eq!(n, 2, "should revoke two of alice's three tokens");

        assert!(
            store.get_token("alice_tok_1").await.unwrap().is_some(),
            "excepted token survives"
        );
        assert!(
            store.get_token("alice_tok_0").await.unwrap().is_none(),
            "non-excepted token revoked"
        );
        assert!(
            store.get_token("alice_tok_2").await.unwrap().is_none(),
            "non-excepted token revoked"
        );
        assert!(
            store.get_token("bob_tok").await.unwrap().is_some(),
            "other user's token untouched"
        );
    }

    // -- DeviceCodeStore --

    #[tokio::test]
    async fn device_code_store_and_get() {
        let layer = setup().await;
        let store = layer.device_code_store();

        let state = DeviceState {
            device_code: "dev_abc".into(),
            user_code: "ABCD-1234".into(),
            client_id: "cli_app".into(),
            scopes: vec!["read".into()],
            expires_at: Utc::now() + Duration::minutes(15),
            authorized_user: None,
        };
        store.store("dev_abc", state).await.unwrap();

        let by_device = store.get_by_device_code("dev_abc").await.unwrap();
        assert!(by_device.is_some());
        assert_eq!(by_device.unwrap().user_code, "ABCD-1234");

        let by_user = store.get_by_user_code("ABCD-1234").await.unwrap();
        assert!(by_user.is_some());
        assert_eq!(by_user.unwrap().device_code, "dev_abc");
    }

    #[tokio::test]
    async fn device_code_update_authorized() {
        let layer = setup().await;
        let store = layer.device_code_store();

        let state = DeviceState {
            device_code: "dev_abc".into(),
            user_code: "ABCD-1234".into(),
            client_id: "cli_app".into(),
            scopes: vec![],
            expires_at: Utc::now() + Duration::minutes(15),
            authorized_user: None,
        };
        store.store("dev_abc", state.clone()).await.unwrap();

        let mut updated = state;
        updated.authorized_user = Some("usr_alice".into());
        store.update("dev_abc", updated).await.unwrap();

        let found = store.get_by_device_code("dev_abc").await.unwrap().unwrap();
        assert_eq!(found.authorized_user, Some("usr_alice".into()));
    }

    #[tokio::test]
    async fn device_code_remove() {
        let layer = setup().await;
        let store = layer.device_code_store();

        let state = DeviceState {
            device_code: "dev_abc".into(),
            user_code: "ABCD-1234".into(),
            client_id: "cli_app".into(),
            scopes: vec![],
            expires_at: Utc::now() + Duration::minutes(15),
            authorized_user: None,
        };
        store.store("dev_abc", state).await.unwrap();
        store.remove("dev_abc").await.unwrap();

        let found = store.get_by_device_code("dev_abc").await.unwrap();
        assert!(found.is_none());
    }
}
