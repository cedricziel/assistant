//! SQLite-backed [`OrgOidcConfigStore`] implementation.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_core::identity::OrgId;
use assistant_core::store::{OrgOidcConfig, OrgOidcConfigStore};

/// SQLite-backed per-org OIDC config store.
pub struct SqliteOrgOidcConfigStore {
    pool: SqlitePool,
}

impl SqliteOrgOidcConfigStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_config(row: sqlx::sqlite::SqliteRow) -> Result<OrgOidcConfig> {
    let domains: String = row.get("allowed_email_domains");
    Ok(OrgOidcConfig {
        org_id: OrgId(row.get("org_id")),
        issuer_url: row.get("issuer_url"),
        client_id: row.get("client_id"),
        client_secret: row.get("client_secret"),
        auto_provision: row.get("auto_provision"),
        allowed_email_domains: serde_json::from_str(&domains)
            .context("parsing allowed_email_domains")?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[async_trait]
impl OrgOidcConfigStore for SqliteOrgOidcConfigStore {
    async fn get_oidc_config(&self, org_id: &OrgId) -> Result<Option<OrgOidcConfig>> {
        let row = sqlx::query(
            "SELECT org_id, issuer_url, client_id, client_secret, auto_provision,
                    allowed_email_domains, created_at, updated_at
             FROM org_oidc_configs WHERE org_id = ?",
        )
        .bind(&org_id.0)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_config).transpose()
    }

    async fn upsert_oidc_config(&self, config: &OrgOidcConfig) -> Result<()> {
        let domains = serde_json::to_string(&config.allowed_email_domains)?;
        sqlx::query(
            "INSERT INTO org_oidc_configs
                (org_id, issuer_url, client_id, client_secret, auto_provision,
                 allowed_email_domains, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(org_id) DO UPDATE SET
                issuer_url = excluded.issuer_url,
                client_id = excluded.client_id,
                client_secret = excluded.client_secret,
                auto_provision = excluded.auto_provision,
                allowed_email_domains = excluded.allowed_email_domains,
                updated_at = excluded.updated_at",
        )
        .bind(&config.org_id.0)
        .bind(&config.issuer_url)
        .bind(&config.client_id)
        .bind(&config.client_secret)
        .bind(config.auto_provision)
        .bind(domains)
        .bind(config.created_at)
        .bind(config.updated_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("saving OIDC config for org {}", config.org_id))?;
        Ok(())
    }

    async fn delete_oidc_config(&self, org_id: &OrgId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM org_oidc_configs WHERE org_id = ?")
            .bind(&org_id.0)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use assistant_core::identity::OrgId;
    use assistant_core::store::{OrgOidcConfig, OrgOidcConfigStore, OrgStore, Organization};

    use crate::org_storage::OrgStorageLayer;

    async fn setup_with_org(org_id: &str) -> OrgStorageLayer {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        let now = Utc::now();
        layer
            .org_store()
            .create_org(&Organization {
                id: OrgId::from(org_id),
                name: "Acme".into(),
                slug: org_id.into(),
                auth_mode: "oidc".into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        layer
    }

    fn make_config(org_id: &str, issuer: &str) -> OrgOidcConfig {
        let now = Utc::now();
        OrgOidcConfig {
            org_id: OrgId::from(org_id),
            issuer_url: issuer.into(),
            client_id: "assistant".into(),
            client_secret: Some("s3cret".into()),
            auto_provision: false,
            allowed_email_domains: vec!["acme.com".into(), "acme.org".into()],
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let layer = setup_with_org("org_1").await;
        let store = layer.oidc_config_store();
        assert!(
            store
                .get_oidc_config(&OrgId::from("org_1"))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn upsert_round_trips_all_fields() {
        let layer = setup_with_org("org_1").await;
        let store = layer.oidc_config_store();
        store
            .upsert_oidc_config(&make_config("org_1", "https://idp.acme.com"))
            .await
            .unwrap();

        let found = store
            .get_oidc_config(&OrgId::from("org_1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.issuer_url, "https://idp.acme.com");
        assert_eq!(found.client_id, "assistant");
        assert_eq!(found.client_secret.as_deref(), Some("s3cret"));
        assert!(!found.auto_provision, "auto_provision should round-trip");
        assert_eq!(
            found.allowed_email_domains,
            vec!["acme.com".to_string(), "acme.org".to_string()]
        );
    }

    #[tokio::test]
    async fn upsert_replaces_existing() {
        let layer = setup_with_org("org_1").await;
        let store = layer.oidc_config_store();
        store
            .upsert_oidc_config(&make_config("org_1", "https://old.example"))
            .await
            .unwrap();
        let mut updated = make_config("org_1", "https://new.example");
        updated.client_secret = None;
        store.upsert_oidc_config(&updated).await.unwrap();

        let found = store
            .get_oidc_config(&OrgId::from("org_1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.issuer_url, "https://new.example");
        assert!(
            found.client_secret.is_none(),
            "upsert should clear the secret"
        );
    }

    #[tokio::test]
    async fn delete_reports_whether_removed() {
        let layer = setup_with_org("org_1").await;
        let store = layer.oidc_config_store();
        store
            .upsert_oidc_config(&make_config("org_1", "https://idp.acme.com"))
            .await
            .unwrap();

        assert!(
            store
                .delete_oidc_config(&OrgId::from("org_1"))
                .await
                .unwrap()
        );
        assert!(
            !store
                .delete_oidc_config(&OrgId::from("org_1"))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn upsert_for_unknown_org_fails() {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        let store = layer.oidc_config_store();
        let result = store
            .upsert_oidc_config(&make_config("ghost", "https://idp.example"))
            .await;
        assert!(result.is_err(), "config must reference an existing org");
    }
}
