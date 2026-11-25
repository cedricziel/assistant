//! Contract tests for [`IntegrationHealthStore`], run against SQLite and in-memory.

use assistant_core::identity::UserId;
use assistant_core::integrations::*;

use super::integration_store::{Fixture, at, integration, memory, org_integration, sqlite};

async fn seeded(fx: Fixture) -> Fixture {
    fx.store
        .create(&org_integration("int_1", "gh"))
        .await
        .unwrap();
    fx
}

async fn nothing_is_known_before_the_first_attempt(fx: Fixture) {
    let fx = seeded(fx).await;
    assert_eq!(fx.health.get("int_1", None).await.unwrap(), None);
    assert_eq!(fx.health.get("missing", None).await.unwrap(), None);
}

async fn a_connection_records_when_and_how_many_tools(fx: Fixture) {
    let fx = seeded(fx).await;
    fx.health
        .record_connected("int_1", None, 7, at(10))
        .await
        .unwrap();

    assert_eq!(
        fx.health.get("int_1", None).await.unwrap(),
        Some(IntegrationHealth {
            integration_id: "int_1".into(),
            subject_user_id: None,
            reachable: true,
            last_connected_at: Some(at(10)),
            tool_count: Some(7),
            last_error: None,
            checked_at: at(10),
        })
    );
}

async fn a_failure_keeps_the_last_good_connection(fx: Fixture) {
    let fx = seeded(fx).await;
    fx.health
        .record_connected("int_1", None, 7, at(10))
        .await
        .unwrap();
    fx.health
        .record_failed("int_1", None, "connect timed out", at(20))
        .await
        .unwrap();

    let health = fx.health.get("int_1", None).await.unwrap().unwrap();
    assert!(!health.reachable);
    assert_eq!(health.last_error.as_deref(), Some("connect timed out"));
    assert_eq!(health.last_connected_at, Some(at(10)));
    assert_eq!(health.tool_count, Some(7));
    assert_eq!(health.checked_at, at(20));

    fx.health
        .record_connected("int_1", None, 8, at(30))
        .await
        .unwrap();
    let health = fx.health.get("int_1", None).await.unwrap().unwrap();
    assert!(health.reachable);
    assert_eq!(health.last_error, None, "a success clears the error");
    assert_eq!(health.tool_count, Some(8));
}

async fn a_failure_before_any_success_has_no_connection_time(fx: Fixture) {
    let fx = seeded(fx).await;
    fx.health
        .record_failed("int_1", None, "refused", at(5))
        .await
        .unwrap();
    let health = fx.health.get("int_1", None).await.unwrap().unwrap();
    assert!(!health.reachable);
    assert_eq!(health.last_connected_at, None);
    assert_eq!(health.tool_count, None);
}

async fn subjects_are_independent(fx: Fixture) {
    let fx = seeded(fx).await;
    let u1 = UserId::from("usr_1");
    fx.health
        .record_connected("int_1", Some(&u1), 3, at(1))
        .await
        .unwrap();
    fx.health
        .record_failed("int_1", None, "down", at(2))
        .await
        .unwrap();

    assert!(
        fx.health
            .get("int_1", Some(&u1))
            .await
            .unwrap()
            .unwrap()
            .reachable
    );
    assert!(
        !fx.health
            .get("int_1", None)
            .await
            .unwrap()
            .unwrap()
            .reachable
    );
    assert_eq!(
        fx.health
            .get("int_1", Some(&UserId::from("usr_2")))
            .await
            .unwrap(),
        None
    );
}

async fn deleting_the_integration_drops_its_health(fx: Fixture) {
    let fx = seeded(fx).await;
    fx.health
        .record_connected("int_1", None, 1, at(1))
        .await
        .unwrap();
    assert!(fx.store.delete("int_1").await.unwrap());
    assert_eq!(fx.health.get("int_1", None).await.unwrap(), None);
}

async fn moving_a_custom_url_forgets_the_old_hosts_health(fx: Fixture) {
    let fx = fx;
    let mut custom = integration("int_c", IntegrationLayer::Org, None, None, "custom");
    custom.catalog_entry_id = None;
    fx.store.create(&custom).await.unwrap();
    fx.health
        .record_connected("int_c", None, 1, at(1))
        .await
        .unwrap();

    custom.connect_url = "https://other.example.com/mcp".into();
    assert!(fx.store.update(&custom).await.unwrap());
    assert_eq!(fx.health.get("int_c", None).await.unwrap(), None);
}

async fn health_needs_an_existing_integration(fx: Fixture) {
    assert!(
        fx.health
            .record_connected("missing", None, 1, at(1))
            .await
            .is_err()
    );
    assert!(
        fx.health
            .record_failed("missing", None, "x", at(1))
            .await
            .is_err()
    );
}

macro_rules! contract_tests {
    ($mod_name:ident, $ctor:expr) => {
        mod $mod_name {
            use super::*;

            #[tokio::test]
            async fn t_nothing_known() {
                nothing_is_known_before_the_first_attempt($ctor).await;
            }
            #[tokio::test]
            async fn t_connected() {
                a_connection_records_when_and_how_many_tools($ctor).await;
            }
            #[tokio::test]
            async fn t_failure_keeps_last_good() {
                a_failure_keeps_the_last_good_connection($ctor).await;
            }
            #[tokio::test]
            async fn t_failure_first() {
                a_failure_before_any_success_has_no_connection_time($ctor).await;
            }
            #[tokio::test]
            async fn t_subjects() {
                subjects_are_independent($ctor).await;
            }
            #[tokio::test]
            async fn t_delete_cascades() {
                deleting_the_integration_drops_its_health($ctor).await;
            }
            #[tokio::test]
            async fn t_url_move_resets() {
                moving_a_custom_url_forgets_the_old_hosts_health($ctor).await;
            }
            #[tokio::test]
            async fn t_requires_integration() {
                health_needs_an_existing_integration($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
