//! [`EffectiveIntegrations`]: which integrations a turn may use.
//!
//! Pure data: it queries the stores and applies the availability, policy and
//! credential rules. It opens no MCP connections.

use std::sync::Arc;

use anyhow::Result;

use super::model::*;
use crate::identity::{OrgId, SpaceId, UserId};

/// Why an integration is, or is not, usable by a caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Available,
    Disabled,
    BlockedByPolicy,
    /// Needs a credential the caller does not have (or no caller, for per-user mode).
    NeedsConnection,
    /// The caller's credential exists but is marked unusable.
    AuthExpired,
}

/// A candidate integration with the caller-specific verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedIntegration {
    pub integration: Integration,
    pub availability: Availability,
    /// Whose credential to use: `None` is the shared credential (or none is needed).
    pub credential_subject: Option<UserId>,
}

/// An integration a turn may use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveIntegration {
    pub integration: Integration,
    /// Whose credential to use: `None` is the shared credential (or none is needed).
    pub credential_subject: Option<UserId>,
}

impl EffectiveIntegration {
    pub fn layer(&self) -> IntegrationLayer {
        self.integration.layer
    }

    pub fn slug(&self) -> &str {
        &self.integration.slug
    }
}

pub struct EffectiveIntegrations {
    integrations: Arc<dyn IntegrationStore>,
    credentials: Arc<dyn IntegrationCredentialStore>,
}

impl EffectiveIntegrations {
    pub fn new(
        integrations: Arc<dyn IntegrationStore>,
        credentials: Arc<dyn IntegrationCredentialStore>,
    ) -> Self {
        Self {
            integrations,
            credentials,
        }
    }

    /// Every candidate for the caller with its verdict, in layer order (org,
    /// space, user): org-layer integrations available in `space`, the space's
    /// own, and `user`'s own.
    ///
    /// A system turn (`user = None`) has no user-layer candidates, and
    /// per-user integrations report [`Availability::NeedsConnection`].
    pub async fn evaluate(
        &self,
        org: &OrgId,
        space: &SpaceId,
        user: Option<&UserId>,
    ) -> Result<Vec<EvaluatedIntegration>> {
        let space_filter = IntegrationFilter {
            layer: Some(IntegrationLayer::Space),
            space_id: Some(space.clone()),
            owner_user_id: None,
        };
        let user_filter = user.map(|user| IntegrationFilter {
            layer: Some(IntegrationLayer::User),
            space_id: None,
            owner_user_id: Some(user.clone()),
        });
        let (policy, mut candidates, space_own, user_own) = futures::try_join!(
            self.integrations.get_policy(org),
            self.org_candidates(org, space),
            self.integrations.list(org, &space_filter),
            async {
                match &user_filter {
                    Some(filter) => self.integrations.list(org, filter).await,
                    None => Ok(Vec::new()),
                }
            },
        )?;
        candidates.extend(space_own);
        candidates.extend(user_own);

        let mut evaluated = Vec::with_capacity(candidates.len());
        for integration in candidates {
            let (availability, credential_subject) =
                self.verdict(&policy, &integration, user).await?;
            evaluated.push(EvaluatedIntegration {
                integration,
                availability,
                credential_subject,
            });
        }
        Ok(evaluated)
    }

    /// The integrations a turn by `user` (or the system, if `None`) in `space` may use.
    pub async fn resolve(
        &self,
        org: &OrgId,
        space: &SpaceId,
        user: Option<&UserId>,
    ) -> Result<Vec<EffectiveIntegration>> {
        Ok(self
            .evaluate(org, space, user)
            .await?
            .into_iter()
            .filter(|e| e.availability == Availability::Available)
            .map(|e| EffectiveIntegration {
                integration: e.integration,
                credential_subject: e.credential_subject,
            })
            .collect())
    }

    /// Org-layer integrations that apply to `space`: with no `allow` rows, all
    /// spaces; otherwise only the listed ones. A space opt-out always wins.
    async fn org_candidates(&self, org: &OrgId, space: &SpaceId) -> Result<Vec<Integration>> {
        let org_layer = IntegrationFilter {
            layer: Some(IntegrationLayer::Org),
            ..IntegrationFilter::default()
        };
        let (integrations, access) = futures::try_join!(
            self.integrations.list(org, &org_layer),
            self.integrations.list_org_space_access(org),
        )?;

        Ok(integrations
            .into_iter()
            .filter(|i| {
                let rows = || access.iter().filter(|a| a.integration_id == i.id);
                let opted_out =
                    rows().any(|a| a.mode == SpaceAccessMode::OptOut && &a.space_id == space);
                let restricted = rows().any(|a| a.mode == SpaceAccessMode::Allow);
                let allowed = !restricted
                    || rows().any(|a| a.mode == SpaceAccessMode::Allow && &a.space_id == space);
                allowed && !opted_out
            })
            .collect())
    }

    async fn verdict(
        &self,
        policy: &IntegrationPolicy,
        integration: &Integration,
        user: Option<&UserId>,
    ) -> Result<(Availability, Option<UserId>)> {
        if !policy.permits(integration) {
            return Ok((Availability::BlockedByPolicy, None));
        }
        if !integration.enabled {
            return Ok((Availability::Disabled, None));
        }
        let subject = match integration.credential_mode {
            CredentialMode::Shared => None,
            CredentialMode::PerUser => match user {
                Some(user) => Some(user.clone()),
                None => return Ok((Availability::NeedsConnection, None)),
            },
        };
        if integration.auth_kind == IntegrationAuthKind::None {
            return Ok((Availability::Available, None));
        }
        let availability = match self
            .credentials
            .get(&integration.id, subject.as_ref())
            .await?
        {
            None => Availability::NeedsConnection,
            Some(c) if c.status == CredentialStatus::Ready => Availability::Available,
            Some(_) => Availability::AuthExpired,
        };
        Ok((availability, subject))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};

    use super::*;
    use crate::integrations::memory::InMemoryIntegrationStore;

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap()
    }

    fn org() -> OrgId {
        OrgId::from("org_1")
    }
    fn eng() -> SpaceId {
        SpaceId::from("sp_eng")
    }
    fn sales() -> SpaceId {
        SpaceId::from("sp_sales")
    }
    fn u1() -> UserId {
        UserId::from("usr_1")
    }
    fn u2() -> UserId {
        UserId::from("usr_2")
    }

    struct World {
        store: Arc<InMemoryIntegrationStore>,
        sut: EffectiveIntegrations,
        next: i64,
    }

    impl World {
        fn new() -> Self {
            let store = Arc::new(InMemoryIntegrationStore::new());
            let sut = EffectiveIntegrations::new(store.clone(), store.credential_store());
            Self {
                store,
                sut,
                next: 0,
            }
        }

        /// Add an integration. Defaults: custom-URL-free catalog install, header auth, shared mode.
        async fn add(
            &mut self,
            id: &str,
            layer: IntegrationLayer,
            space: Option<&SpaceId>,
            owner: Option<&UserId>,
            mutate: impl FnOnce(&mut Integration),
        ) -> Integration {
            self.next += 1;
            let mut i = Integration {
                id: id.into(),
                org_id: org(),
                layer,
                space_id: space.cloned(),
                owner_user_id: owner.cloned(),
                catalog_entry_id: Some("curated/linear-app-mcp".into()),
                connect_url: "https://mcp.example.com".into(),
                slug: id.into(),
                name: id.into(),
                auth_kind: IntegrationAuthKind::Header,
                header_name: Some("Authorization".into()),
                header_value_prefix: Some("Bearer ".into()),
                credential_mode: if layer == IntegrationLayer::User {
                    CredentialMode::PerUser
                } else {
                    CredentialMode::Shared
                },
                trust: IntegrationTrust::Confirm,
                enabled: true,
                oauth_client_id_override: None,
                created_at: at(self.next),
                updated_at: at(self.next),
            };
            mutate(&mut i);
            self.store.create(&i).await.unwrap();
            i
        }

        async fn org_int(&mut self, id: &str, mutate: impl FnOnce(&mut Integration)) {
            self.add(id, IntegrationLayer::Org, None, None, mutate)
                .await;
        }
        async fn space_int(
            &mut self,
            id: &str,
            space: &SpaceId,
            mutate: impl FnOnce(&mut Integration),
        ) {
            self.add(id, IntegrationLayer::Space, Some(space), None, mutate)
                .await;
        }
        async fn user_int(
            &mut self,
            id: &str,
            owner: &UserId,
            mutate: impl FnOnce(&mut Integration),
        ) {
            self.add(id, IntegrationLayer::User, None, Some(owner), mutate)
                .await;
        }

        async fn credential(&self, id: &str, subject: Option<&UserId>, status: CredentialStatus) {
            self.store
                .credential_store()
                .put(&StoredCredential {
                    integration_id: id.into(),
                    subject_user_id: subject.cloned(),
                    kind: CredentialKind::Header,
                    ciphertext: vec![1],
                    nonce: vec![0; 12],
                    key_id: "k".into(),
                    expires_at: None,
                    status,
                    updated_at: at(0),
                })
                .await
                .unwrap();
        }

        async fn ready(&self, id: &str, subject: Option<&UserId>) {
            self.credential(id, subject, CredentialStatus::Ready).await;
        }

        async fn policy(&self, f: impl FnOnce(&mut IntegrationPolicy)) {
            let mut p = IntegrationPolicy::default_for(org());
            f(&mut p);
            self.store.put_policy(&p).await.unwrap();
        }

        async fn resolved(&self, space: &SpaceId, user: Option<&UserId>) -> Vec<String> {
            self.sut
                .resolve(&org(), space, user)
                .await
                .unwrap()
                .into_iter()
                .map(|e| e.integration.id)
                .collect()
        }

        async fn verdict(
            &self,
            id: &str,
            space: &SpaceId,
            user: Option<&UserId>,
        ) -> Option<Availability> {
            self.sut
                .evaluate(&org(), space, user)
                .await
                .unwrap()
                .into_iter()
                .find(|e| e.integration.id == id)
                .map(|e| e.availability)
        }
    }

    #[tokio::test]
    async fn same_service_at_every_layer_yields_one_entry_per_layer_with_credential_subjects() {
        let mut w = World::new();
        w.org_int("org_linear", |i| i.slug = "linear".into()).await;
        w.space_int("space_linear", &eng(), |i| i.slug = "linear".into())
            .await;
        w.user_int("user_linear", &u1(), |i| i.slug = "linear".into())
            .await;
        w.ready("org_linear", None).await;
        w.ready("space_linear", None).await;
        w.ready("user_linear", Some(&u1())).await;

        let got = w.sut.resolve(&org(), &eng(), Some(&u1())).await.unwrap();
        let summary: Vec<_> = got
            .iter()
            .map(|e| (e.layer(), e.slug().to_owned(), e.credential_subject.clone()))
            .collect();
        assert_eq!(
            summary,
            vec![
                (IntegrationLayer::Org, "linear".to_owned(), None),
                (IntegrationLayer::Space, "linear".to_owned(), None),
                (IntegrationLayer::User, "linear".to_owned(), Some(u1())),
            ]
        );
    }

    #[tokio::test]
    async fn org_integration_without_allow_rows_is_available_in_every_space() {
        let mut w = World::new();
        w.org_int("gh", |_| {}).await;
        w.ready("gh", None).await;
        assert_eq!(w.resolved(&eng(), Some(&u1())).await, ["gh"]);
        assert_eq!(w.resolved(&sales(), Some(&u1())).await, ["gh"]);
    }

    #[tokio::test]
    async fn allow_rows_restrict_an_org_integration_to_the_listed_spaces() {
        let mut w = World::new();
        w.org_int("gh", |_| {}).await;
        w.ready("gh", None).await;
        w.store
            .add_space_access("gh", &eng(), SpaceAccessMode::Allow)
            .await
            .unwrap();
        assert_eq!(w.resolved(&eng(), Some(&u1())).await, ["gh"]);
        assert!(w.resolved(&sales(), Some(&u1())).await.is_empty());
    }

    #[tokio::test]
    async fn opt_out_removes_an_org_integration_from_that_space_only() {
        let mut w = World::new();
        w.org_int("gh", |_| {}).await;
        w.ready("gh", None).await;
        w.store
            .add_space_access("gh", &sales(), SpaceAccessMode::OptOut)
            .await
            .unwrap();
        assert!(w.resolved(&sales(), Some(&u1())).await.is_empty());
        assert_eq!(w.resolved(&eng(), Some(&u1())).await, ["gh"]);

        // Opting back in restores it.
        w.store
            .remove_space_access("gh", &sales(), SpaceAccessMode::OptOut)
            .await
            .unwrap();
        assert_eq!(w.resolved(&sales(), Some(&u1())).await, ["gh"]);
    }

    #[tokio::test]
    async fn opt_out_wins_over_an_allow_row_and_opting_back_in_restores_the_allow() {
        let mut w = World::new();
        w.org_int("gh", |_| {}).await;
        w.ready("gh", None).await;
        w.store
            .add_space_access("gh", &eng(), SpaceAccessMode::Allow)
            .await
            .unwrap();
        w.store
            .add_space_access("gh", &eng(), SpaceAccessMode::OptOut)
            .await
            .unwrap();
        assert!(w.resolved(&eng(), Some(&u1())).await.is_empty());
        w.store
            .remove_space_access("gh", &eng(), SpaceAccessMode::OptOut)
            .await
            .unwrap();
        assert_eq!(w.resolved(&eng(), Some(&u1())).await, ["gh"]);
    }

    #[tokio::test]
    async fn a_space_sees_only_its_own_space_layer_integrations() {
        let mut w = World::new();
        w.space_int("eng_sentry", &eng(), |_| {}).await;
        w.space_int("sales_crm", &sales(), |_| {}).await;
        w.ready("eng_sentry", None).await;
        w.ready("sales_crm", None).await;
        assert_eq!(w.resolved(&eng(), Some(&u1())).await, ["eng_sentry"]);
        assert_eq!(w.resolved(&sales(), Some(&u1())).await, ["sales_crm"]);
    }

    #[tokio::test]
    async fn a_user_never_sees_another_users_personal_integrations() {
        let mut w = World::new();
        w.user_int("u1_notion", &u1(), |_| {}).await;
        w.user_int("u2_notion", &u2(), |i| i.slug = "notion2".into())
            .await;
        w.ready("u1_notion", Some(&u1())).await;
        w.ready("u2_notion", Some(&u2())).await;
        assert_eq!(w.resolved(&eng(), Some(&u1())).await, ["u1_notion"]);
        assert_eq!(w.resolved(&eng(), Some(&u2())).await, ["u2_notion"]);
    }

    #[tokio::test]
    async fn other_orgs_integrations_are_never_candidates() {
        let mut w = World::new();
        w.org_int("mine", |_| {}).await;
        w.ready("mine", None).await;
        let other = OrgId::from("org_2");
        let got = w.sut.resolve(&other, &eng(), Some(&u1())).await.unwrap();
        assert!(got.is_empty());
    }

    #[tokio::test]
    async fn disabled_integrations_are_excluded() {
        let mut w = World::new();
        w.space_int("sentry", &eng(), |i| i.enabled = false).await;
        w.ready("sentry", None).await;
        assert!(w.resolved(&eng(), Some(&u1())).await.is_empty());
        assert_eq!(
            w.verdict("sentry", &eng(), Some(&u1())).await,
            Some(Availability::Disabled)
        );
    }

    #[tokio::test]
    async fn per_user_integration_needs_the_callers_own_usable_credential() {
        let mut w = World::new();
        w.org_int("linear", |i| i.credential_mode = CredentialMode::PerUser)
            .await;
        w.ready("linear", Some(&u1())).await;
        w.credential("linear", Some(&u2()), CredentialStatus::AuthExpired)
            .await;

        assert_eq!(w.resolved(&eng(), Some(&u1())).await, ["linear"]);
        let got = w.sut.resolve(&org(), &eng(), Some(&u1())).await.unwrap();
        assert_eq!(got[0].credential_subject, Some(u1()), "acts as the caller");

        let u3 = UserId::from("usr_3");
        assert!(
            w.resolved(&eng(), Some(&u3)).await.is_empty(),
            "never connected"
        );
        assert_eq!(
            w.verdict("linear", &eng(), Some(&u3)).await,
            Some(Availability::NeedsConnection)
        );
        assert!(
            w.resolved(&eng(), Some(&u2())).await.is_empty(),
            "expired credential is unusable"
        );
        assert_eq!(
            w.verdict("linear", &eng(), Some(&u2())).await,
            Some(Availability::AuthExpired)
        );
    }

    #[tokio::test]
    async fn a_shared_credential_does_not_satisfy_per_user_mode_and_vice_versa() {
        let mut w = World::new();
        w.org_int("per_user", |i| i.credential_mode = CredentialMode::PerUser)
            .await;
        w.org_int("shared", |_| {}).await;
        w.ready("per_user", None).await; // wrong subject kind
        w.ready("shared", Some(&u1())).await; // wrong subject kind
        assert!(w.resolved(&eng(), Some(&u1())).await.is_empty());
    }

    #[tokio::test]
    async fn shared_integration_needs_a_usable_shared_credential() {
        let mut w = World::new();
        w.space_int("sentry", &eng(), |_| {}).await;
        assert_eq!(
            w.verdict("sentry", &eng(), Some(&u1())).await,
            Some(Availability::NeedsConnection)
        );
        w.credential("sentry", None, CredentialStatus::AuthExpired)
            .await;
        assert_eq!(
            w.verdict("sentry", &eng(), Some(&u1())).await,
            Some(Availability::AuthExpired)
        );
        w.ready("sentry", None).await;
        assert_eq!(w.resolved(&eng(), Some(&u1())).await, ["sentry"]);
    }

    #[tokio::test]
    async fn integrations_without_auth_need_no_credential_and_use_no_subject() {
        let mut w = World::new();
        w.space_int("open_shared", &eng(), |i| {
            i.auth_kind = IntegrationAuthKind::None
        })
        .await;
        w.org_int("open_per_user", |i| {
            i.auth_kind = IntegrationAuthKind::None;
            i.credential_mode = CredentialMode::PerUser;
        })
        .await;
        let got = w.sut.resolve(&org(), &eng(), Some(&u1())).await.unwrap();
        assert_eq!(got.len(), 2);
        assert!(got.iter().all(|e| e.credential_subject.is_none()));
    }

    #[tokio::test]
    async fn system_turn_includes_only_shared_integrations() {
        let mut w = World::new();
        w.space_int("sentry", &eng(), |_| {}).await;
        w.org_int("linear", |i| i.credential_mode = CredentialMode::PerUser)
            .await;
        w.org_int("open_per_user", |i| {
            i.auth_kind = IntegrationAuthKind::None;
            i.credential_mode = CredentialMode::PerUser;
        })
        .await;
        w.user_int("u1_notion", &u1(), |_| {}).await;
        w.ready("sentry", None).await;
        w.ready("linear", Some(&u1())).await;
        w.ready("u1_notion", Some(&u1())).await;

        assert_eq!(w.resolved(&eng(), None).await, ["sentry"]);
        assert_eq!(
            w.verdict("linear", &eng(), None).await,
            Some(Availability::NeedsConnection)
        );
    }

    #[tokio::test]
    async fn policy_blocks_space_and_user_layer_installs_but_never_org_layer() {
        // (name, layer, catalog entry, policy change, blocked?)
        type Tweak = fn(&mut IntegrationPolicy);
        let cases: Vec<(&str, IntegrationLayer, Option<&str>, Tweak, bool)> = vec![
            (
                "catalog_mode none blocks space catalog install",
                IntegrationLayer::Space,
                Some("curated/a"),
                |p| p.catalog_mode = CatalogMode::None,
                true,
            ),
            (
                "catalog_mode none blocks user catalog install",
                IntegrationLayer::User,
                Some("curated/a"),
                |p| p.catalog_mode = CatalogMode::None,
                true,
            ),
            (
                "catalog_mode none never blocks org layer",
                IntegrationLayer::Org,
                Some("curated/a"),
                |p| p.catalog_mode = CatalogMode::None,
                false,
            ),
            (
                "allowlist blocks an unlisted entry",
                IntegrationLayer::Space,
                Some("curated/a"),
                |p| {
                    p.catalog_mode = CatalogMode::Allowlist;
                    p.allowed_catalog_ids = vec!["curated/b".into()];
                },
                true,
            ),
            (
                "allowlist permits a listed entry",
                IntegrationLayer::User,
                Some("curated/a"),
                |p| {
                    p.catalog_mode = CatalogMode::Allowlist;
                    p.allowed_catalog_ids = vec!["curated/a".into()];
                },
                false,
            ),
            (
                "allowlist never blocks org layer",
                IntegrationLayer::Org,
                Some("curated/a"),
                |p| p.catalog_mode = CatalogMode::Allowlist,
                false,
            ),
            (
                "custom url blocked by default",
                IntegrationLayer::Space,
                None,
                |_| {},
                true,
            ),
            (
                "custom url blocked by default for users",
                IntegrationLayer::User,
                None,
                |_| {},
                true,
            ),
            (
                "custom url permitted when allowed",
                IntegrationLayer::Space,
                None,
                |p| p.allow_custom_urls = true,
                false,
            ),
            (
                "custom url never blocks org layer",
                IntegrationLayer::Org,
                None,
                |_| {},
                false,
            ),
            (
                "custom url ignores catalog_mode none when allowed",
                IntegrationLayer::User,
                None,
                |p| {
                    p.allow_custom_urls = true;
                    p.catalog_mode = CatalogMode::None;
                },
                false,
            ),
            (
                "user integrations disabled",
                IntegrationLayer::User,
                Some("curated/a"),
                |p| p.allow_user_integrations = false,
                true,
            ),
            (
                "user integrations disabled leaves space layer",
                IntegrationLayer::Space,
                Some("curated/a"),
                |p| p.allow_user_integrations = false,
                false,
            ),
            (
                "default policy permits catalog installs",
                IntegrationLayer::User,
                Some("curated/a"),
                |_| {},
                false,
            ),
        ];

        for (name, layer, entry, tweak, blocked) in cases {
            let mut w = World::new();
            let space = (layer == IntegrationLayer::Space).then(eng);
            let owner = (layer == IntegrationLayer::User).then(u1);
            w.add("x", layer, space.as_ref(), owner.as_ref(), |i| {
                i.catalog_entry_id = entry.map(str::to_owned);
                i.auth_kind = IntegrationAuthKind::None;
            })
            .await;
            w.policy(tweak).await;

            let verdict = w.verdict("x", &eng(), Some(&u1())).await;
            let expected = if blocked {
                Availability::BlockedByPolicy
            } else {
                Availability::Available
            };
            assert_eq!(verdict, Some(expected), "{name}");
            assert_eq!(
                w.resolved(&eng(), Some(&u1())).await.is_empty(),
                blocked,
                "{name}: resolve must omit blocked integrations"
            );
        }
    }

    #[tokio::test]
    async fn blocked_by_policy_takes_precedence_over_disabled() {
        let mut w = World::new();
        w.user_int("x", &u1(), |i| i.enabled = false).await;
        w.policy(|p| p.allow_user_integrations = false).await;
        assert_eq!(
            w.verdict("x", &eng(), Some(&u1())).await,
            Some(Availability::BlockedByPolicy)
        );
    }

    #[tokio::test]
    async fn evaluate_reports_every_candidate_in_layer_order() {
        let mut w = World::new();
        w.user_int("u", &u1(), |_| {}).await;
        w.space_int("s", &eng(), |i| i.enabled = false).await;
        w.org_int("o", |_| {}).await;
        let got = w.sut.evaluate(&org(), &eng(), Some(&u1())).await.unwrap();
        let ids: Vec<_> = got.iter().map(|e| e.integration.id.as_str()).collect();
        assert_eq!(ids, ["o", "s", "u"], "org, then space, then user");
    }
}
