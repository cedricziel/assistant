//! Per-org OIDC provider registry.
//!
//! Each org that signs in through an external IdP stores its own
//! [`OrgOidcConfig`]. The registry turns a config into a discovered
//! [`OidcProvider`] and caches it until the config changes, so the IdP's
//! discovery document and JWKS are not re-fetched on every login.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{error, info};

use assistant_auth::oidc::{OidcConfig, OidcProvider};
use assistant_core::identity::OrgId;
use assistant_core::integrations::{SecretBox, oidc_client_secret_aad};
use assistant_core::store::{OrgOidcConfig, OrgOidcConfigStore};
use assistant_storage::OrgStorageLayer;

/// A provider paired with the `updated_at` of the config it was discovered from.
type CachedProvider = (DateTime<Utc>, Arc<OidcProvider>);

/// Resolves the [`OidcProvider`] for an org from its stored config.
pub struct OidcProviderRegistry {
    storage: Arc<OrgStorageLayer>,
    secrets: Arc<SecretBox>,
    http_client: reqwest::Client,
    cache: RwLock<HashMap<OrgId, CachedProvider>>,
}

impl OidcProviderRegistry {
    pub fn new(storage: Arc<OrgStorageLayer>, secrets: Arc<SecretBox>) -> Self {
        Self::with_client(storage, secrets, reqwest::Client::new())
    }

    /// Use a custom HTTP client for discovery (for testing).
    pub fn with_client(
        storage: Arc<OrgStorageLayer>,
        secrets: Arc<SecretBox>,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            storage,
            secrets,
            http_client,
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// The provider for `org_id`, or `None` when the org has no OIDC config.
    ///
    /// A failed discovery, or a client secret the server key cannot open, is
    /// returned as an error and not cached, so the next call retries.
    pub async fn provider_for(&self, org_id: &OrgId) -> Result<Option<Arc<OidcProvider>>> {
        let Some(config) = self
            .storage
            .oidc_config_store()
            .get_oidc_config(org_id)
            .await
            .context("failed to load OIDC config")?
        else {
            self.cache.write().await.remove(org_id);
            return Ok(None);
        };

        if let Some((updated_at, provider)) = self.cache.read().await.get(org_id)
            && *updated_at == config.updated_at
        {
            return Ok(Some(provider.clone()));
        }

        let updated_at = config.updated_at;
        let issuer = config.issuer_url.clone();
        let oidc_config = to_oidc_config(config, &self.secrets).inspect_err(|e| {
            error!(org_id = %org_id, "cannot open the OIDC client secret: {e:#}");
        })?;
        let provider = Arc::new(
            OidcProvider::discover_with_client(oidc_config, self.http_client.clone())
                .await
                .with_context(|| format!("OIDC discovery failed for {issuer}"))?,
        );
        info!(org_id = %org_id, issuer = %issuer, "OIDC provider discovered");
        self.cache
            .write()
            .await
            .insert(org_id.clone(), (updated_at, provider.clone()));
        Ok(Some(provider))
    }
}

fn to_oidc_config(config: OrgOidcConfig, secrets: &SecretBox) -> Result<OidcConfig> {
    let aad = oidc_client_secret_aad(&config.org_id);
    let client_secret = config
        .client_secret
        .map(|sealed| {
            secrets
                .open(&sealed, aad.as_bytes())
                .map(|secret| secret.expose().to_owned())
        })
        .transpose()
        .context("failed to open the OIDC client secret")?;
    Ok(OidcConfig {
        issuer_url: config.issuer_url,
        client_id: config.client_id,
        client_secret,
        auto_provision: config.auto_provision,
        allowed_email_domains: (!config.allowed_email_domains.is_empty())
            .then_some(config.allowed_email_domains),
    })
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use wiremock::MockServer;

    use assistant_core::integrations::SecretBox;

    use super::super::test_support::{
        discovery_hits, mount_idp, oidc_config, sealed_client_secret, seed_oidc_org, seed_org,
        test_secrets,
    };
    use super::*;

    async fn registry() -> (OidcProviderRegistry, Arc<OrgStorageLayer>) {
        let storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        (
            OidcProviderRegistry::new(storage.clone(), test_secrets()),
            storage,
        )
    }

    #[tokio::test]
    async fn org_without_config_has_no_provider() {
        let (registry, storage) = registry().await;
        seed_org(&storage, "org_a", "a", "password").await;

        let provider = registry.provider_for(&OrgId::from("org_a")).await.unwrap();

        assert!(provider.is_none());
    }

    #[tokio::test]
    async fn discovers_provider_from_org_config() {
        let (registry, storage) = registry().await;
        let idp = MockServer::start().await;
        mount_idp(&idp).await;
        seed_oidc_org(&storage, "org_a", "a", &idp.uri(), "client-a").await;

        let provider = registry
            .provider_for(&OrgId::from("org_a"))
            .await
            .unwrap()
            .expect("org has a config");

        assert_eq!(provider.client_id(), "client-a");
        assert_eq!(
            provider.discovery().authorization_endpoint,
            format!("{}/authorize", idp.uri())
        );
    }

    #[tokio::test]
    async fn caches_provider_until_config_changes() {
        let (registry, storage) = registry().await;
        let idp = MockServer::start().await;
        mount_idp(&idp).await;
        seed_oidc_org(&storage, "org_a", "a", &idp.uri(), "client-a").await;
        let org = OrgId::from("org_a");

        let first = registry.provider_for(&org).await.unwrap().unwrap();
        let second = registry.provider_for(&org).await.unwrap().unwrap();

        assert!(Arc::ptr_eq(&first, &second), "second call reuses the cache");
        assert_eq!(discovery_hits(&idp).await, 1, "discovery ran once");
    }

    #[tokio::test]
    async fn rediscovers_when_config_is_updated() {
        let (registry, storage) = registry().await;
        let idp = MockServer::start().await;
        mount_idp(&idp).await;
        seed_oidc_org(&storage, "org_a", "a", &idp.uri(), "client-a").await;
        let org = OrgId::from("org_a");
        let before = registry.provider_for(&org).await.unwrap().unwrap();

        let mut updated = oidc_config(
            "org_a",
            &idp.uri(),
            "client-a2",
            Utc::now() + Duration::seconds(5),
        );
        updated.client_secret = Some(sealed_client_secret(&test_secrets(), "org_a", "s3cret"));
        storage
            .oidc_config_store()
            .upsert_oidc_config(&updated)
            .await
            .unwrap();
        let after = registry.provider_for(&org).await.unwrap().unwrap();

        assert_eq!(before.client_id(), "client-a");
        assert_eq!(after.client_id(), "client-a2");
        assert_eq!(discovery_hits(&idp).await, 2, "update forces rediscovery");
    }

    #[tokio::test]
    async fn failed_discovery_is_an_error_and_not_cached() {
        let (registry, storage) = registry().await;
        let idp = MockServer::start().await;
        seed_oidc_org(&storage, "org_a", "a", &idp.uri(), "client-a").await;
        let org = OrgId::from("org_a");

        assert!(
            registry.provider_for(&org).await.is_err(),
            "unreachable discovery document is an error"
        );

        mount_idp(&idp).await;
        let provider = registry.provider_for(&org).await.unwrap();
        assert!(provider.is_some(), "the next call retries discovery");
    }

    #[tokio::test]
    async fn secret_sealed_under_another_key_is_an_error_and_not_cached() {
        let (registry, storage) = registry().await;
        let idp = MockServer::start().await;
        mount_idp(&idp).await;
        seed_oidc_org(&storage, "org_a", "a", &idp.uri(), "client-a").await;
        let other_key = SecretBox::from_key([9; 32]);
        let mut config = oidc_config("org_a", &idp.uri(), "client-a", Utc::now());
        config.client_secret = Some(sealed_client_secret(&other_key, "org_a", "s3cret"));
        storage
            .oidc_config_store()
            .upsert_oidc_config(&config)
            .await
            .unwrap();
        let org = OrgId::from("org_a");

        assert!(
            registry.provider_for(&org).await.is_err(),
            "a secret the server key cannot open must fail the lookup"
        );
        assert_eq!(
            discovery_hits(&idp).await,
            0,
            "no discovery runs without a usable secret"
        );

        config.client_secret = Some(sealed_client_secret(&test_secrets(), "org_a", "s3cret"));
        storage
            .oidc_config_store()
            .upsert_oidc_config(&config)
            .await
            .unwrap();
        assert!(
            registry.provider_for(&org).await.unwrap().is_some(),
            "the failure was not cached"
        );
    }

    #[test]
    fn opens_the_sealed_secret_for_the_provider_config() {
        let secrets = test_secrets();
        let mut config = oidc_config("org_a", "https://idp.example", "c", Utc::now());
        config.client_secret = Some(sealed_client_secret(&secrets, "org_a", "s3cret"));

        let mapped = to_oidc_config(config.clone(), &secrets).unwrap();
        assert_eq!(mapped.client_secret.as_deref(), Some("s3cret"));

        config.org_id = OrgId::from("org_b");
        assert!(
            to_oidc_config(config, &secrets).is_err(),
            "a secret sealed for another org must not open"
        );
    }

    #[tokio::test]
    async fn deleting_config_drops_cached_provider() {
        let (registry, storage) = registry().await;
        let idp = MockServer::start().await;
        mount_idp(&idp).await;
        seed_oidc_org(&storage, "org_a", "a", &idp.uri(), "client-a").await;
        let org = OrgId::from("org_a");
        registry.provider_for(&org).await.unwrap().unwrap();

        storage
            .oidc_config_store()
            .delete_oidc_config(&org)
            .await
            .unwrap();

        assert!(registry.provider_for(&org).await.unwrap().is_none());
    }

    #[test]
    fn empty_domain_list_means_any_domain() {
        let mut config = oidc_config("org_a", "https://idp.example", "c", Utc::now());
        let secrets = test_secrets();
        assert!(
            to_oidc_config(config.clone(), &secrets)
                .unwrap()
                .allowed_email_domains
                .is_none()
        );

        config.allowed_email_domains = vec!["example.com".into()];
        config.auto_provision = false;
        let mapped = to_oidc_config(config, &secrets).unwrap();
        assert_eq!(
            mapped.allowed_email_domains,
            Some(vec!["example.com".to_string()])
        );
        assert!(!mapped.auto_provision);
    }
}
