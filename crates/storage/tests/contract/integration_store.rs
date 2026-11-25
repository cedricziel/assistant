//! Contract tests for [`IntegrationStore`], run against SQLite and in-memory.

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};

use assistant_core::identity::{OrgId, SpaceId, UserId};
use assistant_core::integrations::*;
use assistant_storage::OrgStorageLayer;

/// A store pair that shares state, so deleting an integration reaches its credentials.
pub struct Fixture {
    pub store: Arc<dyn IntegrationStore>,
    pub credentials: Arc<dyn IntegrationCredentialStore>,
    pub health: Arc<dyn IntegrationHealthStore>,
    /// Kept so the SQLite pool outlives the stores.
    pub layer: Option<OrgStorageLayer>,
}

pub async fn sqlite() -> Fixture {
    let layer = OrgStorageLayer::new_in_memory().await.unwrap();
    for sql in [
        "INSERT INTO organizations (id, name, slug) VALUES ('org_1', 'Acme', 'acme')",
        "INSERT INTO organizations (id, name, slug) VALUES ('org_2', 'Other', 'other')",
        "INSERT INTO users (id, org_id, email) VALUES ('usr_1', 'org_1', 'a@x.io')",
        "INSERT INTO users (id, org_id, email) VALUES ('usr_2', 'org_1', 'b@x.io')",
        "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_eng', 'org_1', 'Eng', 'eng')",
        "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_sales', 'org_1', 'Sales', 'sales')",
    ] {
        sqlx::query(sql).execute(layer.pool()).await.unwrap();
    }
    Fixture {
        store: Arc::new(layer.integration_store()),
        credentials: Arc::new(layer.integration_credential_store()),
        health: Arc::new(layer.integration_health_store()),
        layer: Some(layer),
    }
}

pub fn memory() -> Fixture {
    let store = InMemoryIntegrationStore::new();
    let credentials = store.credential_store();
    let health = store.health_store();
    Fixture {
        store: Arc::new(store),
        credentials,
        health,
        layer: None,
    }
}

pub fn org() -> OrgId {
    OrgId::from("org_1")
}

pub fn at(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap()
}

pub fn integration(
    id: &str,
    layer: IntegrationLayer,
    space: Option<&str>,
    owner: Option<&str>,
    slug: &str,
) -> Integration {
    Integration {
        id: id.into(),
        org_id: org(),
        layer,
        space_id: space.map(SpaceId::from),
        owner_user_id: owner.map(UserId::from),
        catalog_entry_id: Some("curated/linear-app-mcp".into()),
        connect_url: "https://mcp.example.com/mcp".into(),
        slug: slug.into(),
        name: format!("{slug} name"),
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
        created_at: at(0),
        updated_at: at(0),
    }
}

pub fn org_integration(id: &str, slug: &str) -> Integration {
    integration(id, IntegrationLayer::Org, None, None, slug)
}

async fn create_and_get(fx: Fixture) {
    let mut i = integration(
        "int_1",
        IntegrationLayer::Space,
        Some("sp_eng"),
        None,
        "linear",
    );
    i.oauth_client_id_override = Some("client-123".into());
    i.catalog_entry_id = None;
    i.trust = IntegrationTrust::Trust;
    i.enabled = false;
    fx.store.create(&i).await.unwrap();

    assert_eq!(fx.store.get("int_1").await.unwrap(), Some(i));
    assert_eq!(fx.store.get("missing").await.unwrap(), None);
}

async fn duplicate_slug_conflicts_only_within_the_same_scope(fx: Fixture) {
    fx.store
        .create(&org_integration("a", "linear"))
        .await
        .unwrap();
    let dup = fx
        .store
        .create(&org_integration("b", "linear"))
        .await
        .unwrap_err();
    assert_eq!(
        dup.downcast_ref::<IntegrationStoreError>(),
        Some(&IntegrationStoreError::DuplicateSlug {
            slug: "linear".into()
        }),
        "org-layer duplicate: {dup:#}"
    );

    // Same slug in other scopes is fine.
    let ok = [
        integration("c", IntegrationLayer::Space, Some("sp_eng"), None, "linear"),
        integration(
            "d",
            IntegrationLayer::Space,
            Some("sp_sales"),
            None,
            "linear",
        ),
        integration("e", IntegrationLayer::User, None, Some("usr_1"), "linear"),
        integration("f", IntegrationLayer::User, None, Some("usr_2"), "linear"),
    ];
    for i in &ok {
        fx.store.create(i).await.unwrap();
    }
    let dup_space = integration("g", IntegrationLayer::Space, Some("sp_eng"), None, "linear");
    assert!(fx.store.create(&dup_space).await.is_err());
    let dup_user = integration("h", IntegrationLayer::User, None, Some("usr_1"), "linear");
    assert!(fx.store.create(&dup_user).await.is_err());
}

async fn list_filters_by_layer_space_owner_and_org(fx: Fixture) {
    let mut a = org_integration("a", "gh");
    a.created_at = at(1);
    let mut b = integration("b", IntegrationLayer::Space, Some("sp_eng"), None, "gh");
    b.created_at = at(2);
    let mut c = integration("c", IntegrationLayer::Space, Some("sp_sales"), None, "gh");
    c.created_at = at(3);
    let mut d = integration("d", IntegrationLayer::User, None, Some("usr_1"), "gh");
    d.created_at = at(4);
    let mut e = integration("e", IntegrationLayer::User, None, Some("usr_2"), "gh");
    e.created_at = at(5);
    for i in [&e, &d, &c, &b, &a] {
        fx.store.create(i).await.unwrap();
    }

    let ids = |v: Vec<Integration>| v.into_iter().map(|i| i.id).collect::<Vec<_>>();
    let all = fx
        .store
        .list(&org(), &IntegrationFilter::default())
        .await
        .unwrap();
    assert_eq!(ids(all), ["a", "b", "c", "d", "e"], "oldest first");

    let by_layer = |layer| IntegrationFilter {
        layer: Some(layer),
        ..Default::default()
    };
    assert_eq!(
        ids(fx
            .store
            .list(&org(), &by_layer(IntegrationLayer::Org))
            .await
            .unwrap()),
        ["a"]
    );
    assert_eq!(
        ids(fx
            .store
            .list(&org(), &by_layer(IntegrationLayer::Space))
            .await
            .unwrap()),
        ["b", "c"]
    );

    let space = IntegrationFilter {
        layer: Some(IntegrationLayer::Space),
        space_id: Some(SpaceId::from("sp_eng")),
        ..Default::default()
    };
    assert_eq!(ids(fx.store.list(&org(), &space).await.unwrap()), ["b"]);

    let owner = IntegrationFilter {
        layer: Some(IntegrationLayer::User),
        owner_user_id: Some(UserId::from("usr_2")),
        ..Default::default()
    };
    assert_eq!(ids(fx.store.list(&org(), &owner).await.unwrap()), ["e"]);

    let other_org = fx
        .store
        .list(&OrgId::from("org_2"), &IntegrationFilter::default())
        .await
        .unwrap();
    assert!(other_org.is_empty(), "integrations are org-scoped");
}

async fn update_changes_mutable_fields_only(fx: Fixture) {
    let mut original = integration(
        "int_1",
        IntegrationLayer::Space,
        Some("sp_eng"),
        None,
        "linear",
    );
    original.catalog_entry_id = None; // custom URL: connect_url may change
    fx.store.create(&original).await.unwrap();

    let mut edit = original.clone();
    edit.name = "Renamed".into();
    edit.slug = "linear-2".into();
    edit.connect_url = "https://other.example.com/mcp".into();
    edit.trust = IntegrationTrust::Trust;
    edit.enabled = false;
    edit.credential_mode = CredentialMode::PerUser;
    edit.oauth_client_id_override = Some("cid".into());
    edit.updated_at = at(60);
    // Immutable: must be ignored.
    edit.layer = IntegrationLayer::Org;
    edit.space_id = None;
    edit.catalog_entry_id = Some("other/entry".into());
    edit.created_at = at(999);
    assert!(fx.store.update(&edit).await.unwrap());

    let got = fx.store.get("int_1").await.unwrap().unwrap();
    let mut expected = edit.clone();
    expected.layer = original.layer;
    expected.space_id = original.space_id.clone();
    expected.catalog_entry_id = original.catalog_entry_id.clone();
    expected.created_at = original.created_at;
    assert_eq!(got, expected);

    let mut ghost = original.clone();
    ghost.id = "missing".into();
    assert!(!fx.store.update(&ghost).await.unwrap());
}

async fn update_if_unchanged_rejects_a_stale_version(fx: Fixture) {
    let original = integration("int_1", IntegrationLayer::Org, None, None, "linear");
    fx.store.create(&original).await.unwrap();

    let mut first = original.clone();
    first.name = "First".into();
    first.updated_at = at(10);
    assert!(
        fx.store
            .update_if_unchanged(&first, original.updated_at)
            .await
            .unwrap()
    );

    let mut second = original.clone();
    second.enabled = false;
    second.updated_at = at(20);
    let err = fx
        .store
        .update_if_unchanged(&second, original.updated_at)
        .await
        .unwrap_err();
    assert_eq!(
        err.downcast_ref::<IntegrationStoreError>(),
        Some(&IntegrationStoreError::Stale { id: "int_1".into() })
    );
    let got = fx.store.get("int_1").await.unwrap().unwrap();
    assert_eq!(got, first, "the stale write changed nothing");

    let mut ghost = original.clone();
    ghost.id = "missing".into();
    assert!(
        !fx.store
            .update_if_unchanged(&ghost, original.updated_at)
            .await
            .unwrap()
    );
}

async fn update_to_a_taken_slug_conflicts(fx: Fixture) {
    fx.store.create(&org_integration("a", "one")).await.unwrap();
    fx.store.create(&org_integration("b", "two")).await.unwrap();
    let mut b = org_integration("b", "one");
    b.updated_at = at(1);
    let err = fx.store.update(&b).await.unwrap_err();
    assert!(matches!(
        err.downcast_ref::<IntegrationStoreError>(),
        Some(IntegrationStoreError::DuplicateSlug { .. })
    ));
}

async fn delete_cascades_to_credentials_and_space_access(fx: Fixture) {
    fx.store.create(&org_integration("a", "one")).await.unwrap();
    fx.store.create(&org_integration("b", "two")).await.unwrap();
    for id in ["a", "b"] {
        fx.store
            .add_space_access(id, &SpaceId::from("sp_eng"), SpaceAccessMode::Allow)
            .await
            .unwrap();
        fx.credentials
            .put(&super::integration_credential_store::credential(
                id, None, 0,
            ))
            .await
            .unwrap();
        fx.credentials
            .put(&super::integration_credential_store::credential(
                id,
                Some("usr_1"),
                0,
            ))
            .await
            .unwrap();
    }

    assert!(fx.store.delete("a").await.unwrap());
    assert!(
        !fx.store.delete("a").await.unwrap(),
        "second delete reports false"
    );

    assert_eq!(fx.store.get("a").await.unwrap(), None);
    assert!(fx.store.list_space_access("a").await.unwrap().is_empty());
    assert!(fx.credentials.get("a", None).await.unwrap().is_none());
    assert!(
        fx.credentials
            .get("a", Some(&UserId::from("usr_1")))
            .await
            .unwrap()
            .is_none()
    );

    // The other integration is untouched.
    assert!(fx.store.get("b").await.unwrap().is_some());
    assert_eq!(fx.store.list_space_access("b").await.unwrap().len(), 1);
    assert!(fx.credentials.get("b", None).await.unwrap().is_some());
}

async fn space_access_rows_are_idempotent_and_coexist(fx: Fixture) {
    fx.store.create(&org_integration("a", "gh")).await.unwrap();
    let eng = SpaceId::from("sp_eng");
    let sales = SpaceId::from("sp_sales");

    fx.store
        .add_space_access("a", &eng, SpaceAccessMode::Allow)
        .await
        .unwrap();
    fx.store
        .add_space_access("a", &eng, SpaceAccessMode::Allow)
        .await
        .unwrap();
    fx.store
        .add_space_access("a", &eng, SpaceAccessMode::OptOut)
        .await
        .unwrap();
    fx.store
        .add_space_access("a", &sales, SpaceAccessMode::Allow)
        .await
        .unwrap();

    let mut rows = fx.store.list_space_access("a").await.unwrap();
    rows.sort_by_key(|r| (r.space_id.0.clone(), r.mode.as_str()));
    let row = |s: &SpaceId, m| SpaceAccess {
        integration_id: "a".into(),
        space_id: s.clone(),
        mode: m,
    };
    assert_eq!(
        rows,
        vec![
            row(&eng, SpaceAccessMode::Allow),
            row(&eng, SpaceAccessMode::OptOut),
            row(&sales, SpaceAccessMode::Allow),
        ],
        "allow and opt_out may coexist for one space"
    );

    assert!(
        fx.store
            .remove_space_access("a", &eng, SpaceAccessMode::OptOut)
            .await
            .unwrap()
    );
    assert!(
        !fx.store
            .remove_space_access("a", &eng, SpaceAccessMode::OptOut)
            .await
            .unwrap()
    );
    assert_eq!(fx.store.list_space_access("a").await.unwrap().len(), 2);
}

async fn org_space_access_lists_every_integration_of_the_org(fx: Fixture) {
    fx.store.create(&org_integration("a", "one")).await.unwrap();
    fx.store.create(&org_integration("b", "two")).await.unwrap();
    let eng = SpaceId::from("sp_eng");
    fx.store
        .add_space_access("a", &eng, SpaceAccessMode::Allow)
        .await
        .unwrap();
    fx.store
        .add_space_access("b", &eng, SpaceAccessMode::OptOut)
        .await
        .unwrap();

    let rows = fx.store.list_org_space_access(&org()).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        fx.store
            .list_org_space_access(&OrgId::from("org_2"))
            .await
            .unwrap()
            .is_empty()
    );
}

async fn policy_defaults_then_round_trips(fx: Fixture) {
    let default = fx.store.get_policy(&org()).await.unwrap();
    assert_eq!(default.catalog_mode, CatalogMode::All);
    assert!(default.allowed_catalog_ids.is_empty());
    assert!(!default.allow_custom_urls);
    assert!(default.allow_user_integrations);

    let policy = IntegrationPolicy {
        org_id: org(),
        catalog_mode: CatalogMode::Allowlist,
        allowed_catalog_ids: vec!["curated/a".into(), "curated/b".into()],
        allow_custom_urls: true,
        allow_user_integrations: false,
        updated_at: at(5),
    };
    fx.store.put_policy(&policy).await.unwrap();
    assert_eq!(fx.store.get_policy(&org()).await.unwrap(), policy);

    let mut tighter = policy.clone();
    tighter.catalog_mode = CatalogMode::None;
    tighter.updated_at = at(9);
    fx.store.put_policy(&tighter).await.unwrap();
    assert_eq!(
        fx.store.get_policy(&org()).await.unwrap(),
        tighter,
        "put replaces"
    );

    let other = fx.store.get_policy(&OrgId::from("org_2")).await.unwrap();
    assert_eq!(other.catalog_mode, CatalogMode::All, "policy is per org");
}

async fn catalog_install_connect_url_is_immutable(fx: Fixture) {
    let mut i = org_integration("int_1", "linear");
    i.credential_mode = CredentialMode::PerUser;
    fx.store.create(&i).await.unwrap();
    for subject in [None, Some("usr_1")] {
        fx.credentials
            .put(&super::integration_credential_store::credential(
                "int_1", subject, 0,
            ))
            .await
            .unwrap();
    }

    let mut edit = i.clone();
    edit.connect_url = "https://attacker.example/mcp".into();
    edit.name = "Renamed".into();
    let err = fx.store.update(&edit).await.unwrap_err();
    assert_eq!(
        err.downcast_ref::<IntegrationStoreError>(),
        Some(&IntegrationStoreError::ConnectUrlImmutable { id: "int_1".into() }),
        "{err:#}"
    );

    let got = fx.store.get("int_1").await.unwrap().unwrap();
    assert_eq!(got, i, "a rejected update changes nothing");
    assert!(fx.credentials.get("int_1", None).await.unwrap().is_some());
    assert!(
        fx.credentials
            .get("int_1", Some(&UserId::from("usr_1")))
            .await
            .unwrap()
            .is_some()
    );

    // Re-saving the same URL is fine.
    let mut same = i.clone();
    same.name = "Renamed".into();
    same.updated_at = at(5);
    assert!(fx.store.update(&same).await.unwrap());
}

async fn custom_url_change_deletes_every_credential(fx: Fixture) {
    let mut i = org_integration("int_1", "custom");
    i.catalog_entry_id = None;
    i.credential_mode = CredentialMode::PerUser;
    let mut other = org_integration("int_2", "other");
    other.catalog_entry_id = None;
    fx.store.create(&i).await.unwrap();
    fx.store.create(&other).await.unwrap();
    for (id, subject) in [("int_1", None), ("int_1", Some("usr_1")), ("int_2", None)] {
        fx.credentials
            .put(&super::integration_credential_store::credential(
                id, subject, 0,
            ))
            .await
            .unwrap();
    }

    // Unchanged URL keeps credentials.
    let mut rename = i.clone();
    rename.name = "Renamed".into();
    assert!(fx.store.update(&rename).await.unwrap());
    assert!(fx.credentials.get("int_1", None).await.unwrap().is_some());

    let mut moved = i.clone();
    moved.connect_url = "https://new.example/mcp".into();
    moved.updated_at = at(9);
    assert!(fx.store.update(&moved).await.unwrap());
    assert_eq!(
        fx.store.get("int_1").await.unwrap().unwrap().connect_url,
        "https://new.example/mcp"
    );
    assert!(fx.credentials.get("int_1", None).await.unwrap().is_none());
    assert!(
        fx.credentials
            .get("int_1", Some(&UserId::from("usr_1")))
            .await
            .unwrap()
            .is_none(),
        "per-user credentials must go too"
    );
    assert!(
        fx.credentials.get("int_2", None).await.unwrap().is_some(),
        "other integrations keep theirs"
    );
}

async fn create_rejects_rows_that_violate_the_schema_invariants(fx: Fixture) {
    let bad: Vec<(&str, Integration)> = vec![
        (
            "space layer without space_id",
            integration("b1", IntegrationLayer::Space, None, None, "x"),
        ),
        (
            "space layer with owner",
            integration(
                "b2",
                IntegrationLayer::Space,
                Some("sp_eng"),
                Some("usr_1"),
                "x",
            ),
        ),
        (
            "user layer without owner",
            integration("b3", IntegrationLayer::User, None, None, "x"),
        ),
        (
            "user layer with space",
            integration(
                "b4",
                IntegrationLayer::User,
                Some("sp_eng"),
                Some("usr_1"),
                "x",
            ),
        ),
        (
            "org layer with space",
            integration("b5", IntegrationLayer::Org, Some("sp_eng"), None, "x"),
        ),
        (
            "org layer with owner",
            integration("b6", IntegrationLayer::Org, None, Some("usr_1"), "x"),
        ),
        ("empty slug", org_integration("b7", "")),
        ("slug over 32 chars", org_integration("b8", &"a".repeat(33))),
        ("user layer with shared mode", {
            let mut i = integration("b9", IntegrationLayer::User, None, Some("usr_1"), "x");
            i.credential_mode = CredentialMode::Shared;
            i
        }),
    ];
    for (name, i) in bad {
        assert!(
            fx.store.create(&i).await.is_err(),
            "{name} must be rejected"
        );
        assert!(
            fx.store.get(&i.id).await.unwrap().is_none(),
            "{name} was stored"
        );
    }
}

async fn update_rejects_rows_that_violate_the_schema_invariants(fx: Fixture) {
    let user_int = integration("u", IntegrationLayer::User, None, Some("usr_1"), "x");
    fx.store.create(&user_int).await.unwrap();
    let mut shared = user_int.clone();
    shared.credential_mode = CredentialMode::Shared;
    assert!(
        fx.store.update(&shared).await.is_err(),
        "user layer must stay per_user"
    );
    let mut empty = user_int.clone();
    empty.slug = String::new();
    assert!(fx.store.update(&empty).await.is_err(), "empty slug");
    assert_eq!(fx.store.get("u").await.unwrap().unwrap(), user_int);
}

async fn space_access_requires_an_existing_integration(fx: Fixture) {
    let res = fx
        .store
        .add_space_access("missing", &SpaceId::from("sp_eng"), SpaceAccessMode::Allow)
        .await;
    assert!(res.is_err());
}

macro_rules! contract_tests {
    ($mod_name:ident, $ctor:expr) => {
        mod $mod_name {
            use super::*;

            #[tokio::test]
            async fn t_create_and_get() {
                create_and_get($ctor).await;
            }
            #[tokio::test]
            async fn t_duplicate_slug() {
                duplicate_slug_conflicts_only_within_the_same_scope($ctor).await;
            }
            #[tokio::test]
            async fn t_list_filters() {
                list_filters_by_layer_space_owner_and_org($ctor).await;
            }
            #[tokio::test]
            async fn t_update_mutable_only() {
                update_changes_mutable_fields_only($ctor).await;
            }
            #[tokio::test]
            async fn t_update_if_unchanged() {
                update_if_unchanged_rejects_a_stale_version($ctor).await;
            }
            #[tokio::test]
            async fn t_update_slug_conflict() {
                update_to_a_taken_slug_conflicts($ctor).await;
            }
            #[tokio::test]
            async fn t_delete_cascades() {
                delete_cascades_to_credentials_and_space_access($ctor).await;
            }
            #[tokio::test]
            async fn t_space_access() {
                space_access_rows_are_idempotent_and_coexist($ctor).await;
            }
            #[tokio::test]
            async fn t_org_space_access() {
                org_space_access_lists_every_integration_of_the_org($ctor).await;
            }
            #[tokio::test]
            async fn t_catalog_url_immutable() {
                catalog_install_connect_url_is_immutable($ctor).await;
            }
            #[tokio::test]
            async fn t_custom_url_change_drops_credentials() {
                custom_url_change_deletes_every_credential($ctor).await;
            }
            #[tokio::test]
            async fn t_create_invariants() {
                create_rejects_rows_that_violate_the_schema_invariants($ctor).await;
            }
            #[tokio::test]
            async fn t_update_invariants() {
                update_rejects_rows_that_violate_the_schema_invariants($ctor).await;
            }
            #[tokio::test]
            async fn t_space_access_needs_integration() {
                space_access_requires_an_existing_integration($ctor).await;
            }
            #[tokio::test]
            async fn t_policy() {
                policy_defaults_then_round_trips($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
