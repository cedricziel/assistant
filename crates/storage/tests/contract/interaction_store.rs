//! Contract tests for the [`InteractionStore`] trait.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use assistant_core::clock::{Clock, FakeClock};
use assistant_storage::{
    InMemoryInteractionStore, InteractionRow, InteractionStatus, InteractionStore, NewInteraction,
    ResolveOutcome, SqliteInteractionStore, StorageLayer,
};
use chrono::Duration;
use serde_json::json;

struct Harness {
    store: Box<dyn InteractionStore>,
    clock: Arc<FakeClock>,
}

async fn sqlite_harness() -> Harness {
    let clock = Arc::new(FakeClock::new_at_seed());
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let store = SqliteInteractionStore::new(storage.pool).with_clock(clock.clone());
    Harness {
        store: Box::new(store),
        clock,
    }
}

fn memory_harness() -> Harness {
    let clock = Arc::new(FakeClock::new_at_seed());
    let store = InMemoryInteractionStore::new().with_clock(clock.clone());
    Harness {
        store: Box::new(store),
        clock,
    }
}

fn new_interaction(id: &str, conversation_id: &str, run_id: &str) -> NewInteraction {
    NewInteraction {
        id: id.to_string(),
        conversation_id: conversation_id.to_string(),
        run_id: run_id.to_string(),
        user_id: "user-1".to_string(),
        kind: "tool_approval".to_string(),
        tool_call_id: Some(format!("call-{id}")),
        request: json!({ "tool": "bash", "args": { "command": "ls" } }),
        expires_at: FakeClock::new_at_seed().now() + Duration::minutes(10),
    }
}

async fn create_then_get(h: Harness) {
    let created = h
        .store
        .create(new_interaction("i1", "c1", "r1"))
        .await
        .unwrap();
    assert_eq!(created.status, InteractionStatus::Pending);
    assert_eq!(created.created_at, h.clock.now());
    assert_eq!(created.waiter_seen_at, h.clock.now());
    assert!(!created.resolution_emitted);
    assert!(created.response.is_none());
    assert!(created.resolved_at.is_none());

    let fetched = h.store.get("i1").await.unwrap().expect("row exists");
    assert_eq!(fetched, created, "get returns exactly what create returned");
    assert_eq!(fetched.kind, "tool_approval");
    assert_eq!(fetched.tool_call_id.as_deref(), Some("call-i1"));
    assert_eq!(fetched.request["tool"], "bash");

    assert!(h.store.get("missing").await.unwrap().is_none());
}

async fn create_rejects_duplicate_id(h: Harness) {
    h.store
        .create(new_interaction("dup", "c1", "r1"))
        .await
        .unwrap();
    let again = h.store.create(new_interaction("dup", "c1", "r1")).await;
    assert!(again.is_err(), "a second create with the same id must fail");
}

async fn resolve_if_pending_wins_once(h: Harness) {
    h.store
        .create(new_interaction("i1", "c1", "r1"))
        .await
        .unwrap();
    h.clock.advance(StdDuration::from_secs(5));

    let first = h
        .store
        .resolve_if_pending(
            "i1",
            InteractionStatus::Resolved,
            Some(&json!({ "decision": "allow" })),
            Some("rule-9"),
        )
        .await
        .unwrap();
    let ResolveOutcome::Resolved(row) = first else {
        panic!("first resolve must win, got {first:?}");
    };
    assert_eq!(row.status, InteractionStatus::Resolved);
    assert_eq!(row.response, Some(json!({ "decision": "allow" })));
    assert_eq!(row.rule_id.as_deref(), Some("rule-9"));
    assert_eq!(row.resolved_at, Some(h.clock.now()));

    let second = h
        .store
        .resolve_if_pending(
            "i1",
            InteractionStatus::Resolved,
            Some(&json!({ "decision": "deny" })),
            None,
        )
        .await
        .unwrap();
    let ResolveOutcome::AlreadyResolved(existing) = second else {
        panic!("second resolve must lose, got {second:?}");
    };
    assert_eq!(
        existing.response,
        Some(json!({ "decision": "allow" })),
        "the loser sees the winner's stored outcome"
    );
    assert_eq!(existing.rule_id.as_deref(), Some("rule-9"));
}

async fn resolve_missing_row_reports_not_found(h: Harness) {
    let outcome = h
        .store
        .resolve_if_pending("nope", InteractionStatus::Resolved, None, None)
        .await
        .unwrap();
    assert!(matches!(outcome, ResolveOutcome::NotFound), "{outcome:?}");
}

async fn resolve_rejects_pending_target_status(h: Harness) {
    h.store
        .create(new_interaction("i1", "c1", "r1"))
        .await
        .unwrap();
    let result = h
        .store
        .resolve_if_pending("i1", InteractionStatus::Pending, None, None)
        .await;
    assert!(result.is_err(), "resolving to pending makes no sense");
}

async fn list_for_conversation_filters_in_creation_order(h: Harness) {
    for (id, conv) in [("a", "c1"), ("b", "c1"), ("x", "c2"), ("c", "c1")] {
        h.store
            .create(new_interaction(id, conv, "r1"))
            .await
            .unwrap();
        h.clock.advance(StdDuration::from_secs(1));
    }
    h.store
        .resolve_if_pending("b", InteractionStatus::Resolved, None, None)
        .await
        .unwrap();

    let ids = |rows: Vec<InteractionRow>| rows.into_iter().map(|r| r.id).collect::<Vec<_>>();

    let all = ids(h.store.list_for_conversation("c1", None).await.unwrap());
    assert_eq!(all, ["a", "b", "c"]);

    let pending = ids(h
        .store
        .list_for_conversation("c1", Some(InteractionStatus::Pending))
        .await
        .unwrap());
    assert_eq!(pending, ["a", "c"]);

    let resolved = ids(h
        .store
        .list_for_conversation("c1", Some(InteractionStatus::Resolved))
        .await
        .unwrap());
    assert_eq!(resolved, ["b"]);
}

async fn touch_waiter_refreshes_waiter_seen_at(h: Harness) {
    h.store
        .create(new_interaction("i1", "c1", "r1"))
        .await
        .unwrap();
    h.clock.advance(StdDuration::from_secs(30));
    h.store.touch_waiter("i1").await.unwrap();
    let row = h.store.get("i1").await.unwrap().unwrap();
    assert_eq!(row.waiter_seen_at, h.clock.now());
    assert!(row.waiter_seen_at > row.created_at);
}

async fn expire_dead_only_touches_stale_pending_rows(h: Harness) {
    h.store
        .create(new_interaction("stale", "c1", "r1"))
        .await
        .unwrap();
    h.store
        .create(new_interaction("done", "c1", "r1"))
        .await
        .unwrap();
    h.store
        .create(new_interaction("fresh", "c1", "r1"))
        .await
        .unwrap();
    h.store
        .resolve_if_pending("done", InteractionStatus::Resolved, None, None)
        .await
        .unwrap();

    h.clock.advance(StdDuration::from_secs(60));
    h.store.touch_waiter("fresh").await.unwrap();

    let cutoff = h.clock.now() - Duration::seconds(45);
    let expired = h.store.expire_dead(cutoff).await.unwrap();
    assert_eq!(expired, ["stale"]);

    let stale = h.store.get("stale").await.unwrap().unwrap();
    assert_eq!(stale.status, InteractionStatus::Expired);
    assert_eq!(stale.resolved_at, Some(h.clock.now()));
    assert_eq!(
        h.store.get("done").await.unwrap().unwrap().status,
        InteractionStatus::Resolved,
        "terminal rows are never rewritten"
    );
    assert_eq!(
        h.store.get("fresh").await.unwrap().unwrap().status,
        InteractionStatus::Pending,
        "a live waiter keeps its row"
    );
}

async fn cancel_pending_for_run_returns_affected_ids(h: Harness) {
    for (id, run) in [("a", "r1"), ("b", "r1"), ("c", "r1"), ("d", "r2")] {
        h.store
            .create(new_interaction(id, "c1", run))
            .await
            .unwrap();
    }
    h.store
        .resolve_if_pending("b", InteractionStatus::Resolved, None, None)
        .await
        .unwrap();

    let cancelled = h.store.cancel_pending_for_run("r1").await.unwrap();
    assert_eq!(cancelled, ["a", "c"]);

    assert_eq!(
        h.store.get("a").await.unwrap().unwrap().status,
        InteractionStatus::Cancelled
    );
    assert_eq!(
        h.store.get("b").await.unwrap().unwrap().status,
        InteractionStatus::Resolved
    );
    assert_eq!(
        h.store.get("d").await.unwrap().unwrap().status,
        InteractionStatus::Pending,
        "other runs are untouched"
    );
    assert!(
        h.store
            .cancel_pending_for_run("r1")
            .await
            .unwrap()
            .is_empty(),
        "nothing left to cancel"
    );
}

async fn unemitted_rows_include_resolved_until_marked(h: Harness) {
    for (id, run) in [("a", "r1"), ("b", "r1"), ("c", "r1"), ("d", "r2")] {
        h.store
            .create(new_interaction(id, "c1", run))
            .await
            .unwrap();
    }
    h.store
        .resolve_if_pending("b", InteractionStatus::Resolved, None, None)
        .await
        .unwrap();
    h.store.cancel_pending_for_run("r1").await.unwrap();

    let ids = |rows: Vec<InteractionRow>| rows.into_iter().map(|r| r.id).collect::<Vec<_>>();
    assert_eq!(
        ids(h.store.list_unemitted_for_run("r1").await.unwrap()),
        ["a", "b", "c"],
        "resolved and cancelled rows are both reported"
    );

    h.store.mark_emitted("b").await.unwrap();
    assert!(h.store.get("b").await.unwrap().unwrap().resolution_emitted);
    assert_eq!(
        ids(h.store.list_unemitted_for_run("r1").await.unwrap()),
        ["a", "c"]
    );
    assert_eq!(
        ids(h.store.list_unemitted_for_run("r2").await.unwrap()),
        ["d"]
    );
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_create_then_get() {
                create_then_get($ctor).await;
            }

            #[tokio::test]
            async fn t_create_rejects_duplicate_id() {
                create_rejects_duplicate_id($ctor).await;
            }

            #[tokio::test]
            async fn t_resolve_if_pending_wins_once() {
                resolve_if_pending_wins_once($ctor).await;
            }

            #[tokio::test]
            async fn t_resolve_missing_row_reports_not_found() {
                resolve_missing_row_reports_not_found($ctor).await;
            }

            #[tokio::test]
            async fn t_resolve_rejects_pending_target_status() {
                resolve_rejects_pending_target_status($ctor).await;
            }

            #[tokio::test]
            async fn t_list_for_conversation_filters_in_creation_order() {
                list_for_conversation_filters_in_creation_order($ctor).await;
            }

            #[tokio::test]
            async fn t_touch_waiter_refreshes_waiter_seen_at() {
                touch_waiter_refreshes_waiter_seen_at($ctor).await;
            }

            #[tokio::test]
            async fn t_expire_dead_only_touches_stale_pending_rows() {
                expire_dead_only_touches_stale_pending_rows($ctor).await;
            }

            #[tokio::test]
            async fn t_cancel_pending_for_run_returns_affected_ids() {
                cancel_pending_for_run_returns_affected_ids($ctor).await;
            }

            #[tokio::test]
            async fn t_unemitted_rows_include_resolved_until_marked() {
                unemitted_rows_include_resolved_until_marked($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite, sqlite_harness().await);
contract_tests!(memory, memory_harness());
