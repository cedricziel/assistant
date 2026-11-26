//! Contract tests for the [`ConversationStore`] trait.
//!
//! The same test scenarios run against every implementation
//! (`SqliteConversationStore` + `InMemoryConversationStore`).  Any
//! observable divergence between impls fails CI.

use assistant_core::types::conversation::{Message, MessageFinishReason, MessageRole};
use assistant_storage::{
    ConversationPatch, ConversationStore, InMemoryConversationStore, MetadataUpdate,
    SqliteConversationStore, StorageLayer,
};
use uuid::Uuid;

/// Construct a SQLite-backed store for tests.
async fn sqlite_store() -> Box<dyn ConversationStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteConversationStore::new(storage.pool.clone()))
}

/// Construct an in-memory store for tests.
fn memory_store() -> Box<dyn ConversationStore> {
    Box::new(InMemoryConversationStore::new())
}

fn user_msg(conversation_id: Uuid, body: &str, turn: i64) -> Message {
    let mut m = Message::user(conversation_id, body);
    m.turn = turn;
    m
}

fn assistant_msg(conversation_id: Uuid, body: &str, turn: i64) -> Message {
    let mut m = Message::assistant(conversation_id, body);
    m.turn = turn;
    m
}

async fn create_and_get_roundtrip(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Hello")).await.unwrap();
    assert_eq!(conv.title.as_deref(), Some("Hello"));
    assert!(conv.title_locked, "explicit title must lock");

    let loaded = store.get_conversation(conv.id).await.unwrap();
    let loaded = loaded.expect("must round-trip");
    assert_eq!(loaded.id, conv.id);
    assert_eq!(loaded.title.as_deref(), Some("Hello"));
}

async fn create_untitled_does_not_lock(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(None).await.unwrap();
    assert!(conv.title.is_none());
    assert!(!conv.title_locked, "untitled conversations stay unlocked");
}

async fn create_with_id_idempotent(store: Box<dyn ConversationStore>) {
    let id = Uuid::new_v4();
    let first = store
        .create_conversation_with_id(id, Some("First"))
        .await
        .unwrap();
    let second = store
        .create_conversation_with_id(id, Some("Second"))
        .await
        .unwrap();
    assert_eq!(
        first.id, second.id,
        "create_with_id is idempotent for the same UUID"
    );
    // The contract is: if a row already exists, return it unchanged.
    assert_eq!(
        second.title.as_deref(),
        Some("First"),
        "existing row's title survives a second create_with_id"
    );
}

async fn list_orders_by_recency(store: Box<dyn ConversationStore>) {
    let a = store.create_conversation(Some("A")).await.unwrap();
    let b = store.create_conversation(Some("B")).await.unwrap();
    let _c = store.create_conversation(Some("C")).await.unwrap();
    // Touch `a` last so it becomes the most-recent.
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store
        .save_message(&user_msg(a.id, "bump", 0))
        .await
        .unwrap();

    let list = store.list_conversations().await.unwrap();
    assert!(!list.is_empty());
    // The first element should be `a` (just-updated).
    assert_eq!(list[0].id, a.id);
    let _ = b;
}

async fn update_title_locks(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(None).await.unwrap();
    store.update_title(conv.id, "renamed").await.unwrap();
    let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
    assert_eq!(loaded.title.as_deref(), Some("renamed"));
    assert!(loaded.title_locked, "explicit update_title must lock");
}

async fn update_title_does_not_bump_updated_at(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(None).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store.update_title(conv.id, "renamed").await.unwrap();
    let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
    assert_eq!(
        loaded.updated_at, conv.updated_at,
        "renaming must not change updated_at"
    );
}

fn patch(title: Option<&str>, pinned: Option<bool>) -> ConversationPatch {
    ConversationPatch {
        title: title.map(String::from),
        pinned,
    }
}

async fn expect_updated(
    store: &dyn ConversationStore,
    id: Uuid,
    p: ConversationPatch,
) -> assistant_storage::ConversationRecord {
    match store.update_metadata(id, p).await.unwrap() {
        MetadataUpdate::Updated(rec) => rec,
        MetadataUpdate::NotFound => panic!("expected Updated, got NotFound"),
    }
}

async fn update_metadata_title_and_pin(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(None).await.unwrap();
    assert!(conv.pinned_at.is_none(), "new conversations are not pinned");
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    let rec = expect_updated(&*store, conv.id, patch(Some("Renamed"), Some(true))).await;
    assert_eq!(rec.title.as_deref(), Some("Renamed"));
    assert!(rec.title_locked, "a title patch must lock the title");
    assert!(rec.pinned_at.is_some(), "pinned: true must set pinned_at");
    assert_eq!(
        rec.updated_at, conv.updated_at,
        "metadata edits must not change updated_at"
    );

    let reloaded = store.get_conversation(conv.id).await.unwrap().unwrap();
    assert_eq!(reloaded.pinned_at, rec.pinned_at, "pin must be persisted");
}

async fn update_metadata_repin_keeps_pinned_at(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("A")).await.unwrap();
    let first = expect_updated(&*store, conv.id, patch(None, Some(true))).await;
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let second = expect_updated(&*store, conv.id, patch(None, Some(true))).await;
    assert_eq!(
        first.pinned_at, second.pinned_at,
        "re-pinning must keep the original pinned_at"
    );
}

async fn update_metadata_unpin_clears_pinned_at(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("A")).await.unwrap();
    expect_updated(&*store, conv.id, patch(None, Some(true))).await;
    let rec = expect_updated(&*store, conv.id, patch(None, Some(false))).await;
    assert!(
        rec.pinned_at.is_none(),
        "pinned: false must clear pinned_at"
    );
    assert_eq!(
        rec.title.as_deref(),
        Some("A"),
        "an absent title leaves the title unchanged"
    );
}

async fn update_metadata_unknown_id_is_not_found(store: Box<dyn ConversationStore>) {
    let res = store
        .update_metadata(Uuid::new_v4(), patch(Some("x"), None))
        .await
        .unwrap();
    assert!(matches!(res, MetadataUpdate::NotFound));
}

async fn update_metadata_overwrites_locked_title(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Locked")).await.unwrap();
    assert!(conv.title_locked);
    let rec = expect_updated(&*store, conv.id, patch(Some("Newer"), None)).await;
    assert_eq!(rec.title.as_deref(), Some("Newer"));
}

async fn list_orders_pinned_first_newest_pin_first(store: Box<dyn ConversationStore>) {
    let a = store.create_conversation(Some("A")).await.unwrap();
    let b = store.create_conversation(Some("B")).await.unwrap();
    let c = store.create_conversation(Some("C")).await.unwrap();
    let d = store.create_conversation(Some("D")).await.unwrap();
    // `d` is the most recently updated unpinned conversation.
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store
        .save_message(&user_msg(d.id, "bump", 0))
        .await
        .unwrap();

    expect_updated(&*store, a.id, patch(None, Some(true))).await;
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    expect_updated(&*store, b.id, patch(None, Some(true))).await;

    let ids: Vec<Uuid> = store
        .list_conversations()
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(
        ids,
        vec![b.id, a.id, d.id, c.id],
        "pinned first (newest pin first), then by updated_at"
    );
}

async fn set_generated_title_writes_unlocked_row(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(None).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let written = store
        .set_generated_title(conv.id, "Generated")
        .await
        .unwrap();

    assert!(written, "unlocked row must accept the generated title");
    let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
    assert_eq!(loaded.title.as_deref(), Some("Generated"));
    assert!(loaded.title_locked, "generated title must lock the row");
    assert_eq!(
        loaded.updated_at, conv.updated_at,
        "generated title must not change updated_at"
    );
}

async fn set_generated_title_skips_locked_row(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(None).await.unwrap();
    store.update_title(conv.id, "Trip plan").await.unwrap();

    let written = store
        .set_generated_title(conv.id, "Generated")
        .await
        .unwrap();

    assert!(!written, "locked row must reject the generated title");
    let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
    assert_eq!(loaded.title.as_deref(), Some("Trip plan"));
}

async fn set_generated_title_skips_missing_row(store: Box<dyn ConversationStore>) {
    let written = store
        .set_generated_title(Uuid::new_v4(), "Generated")
        .await
        .unwrap();
    assert!(!written, "missing row must report no write");
}

async fn save_and_load_history(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("History")).await.unwrap();
    let u = user_msg(conv.id, "hi", 0);
    let a = assistant_msg(conv.id, "hello!", 0);
    store.save_message(&u).await.unwrap();
    store.save_message(&a).await.unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert!(matches!(history[0].role, MessageRole::User));
    assert!(matches!(history[1].role, MessageRole::Assistant));
    assert_eq!(history[0].content, "hi");
    assert_eq!(history[1].content, "hello!");
}

async fn get_message_by_id(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("M")).await.unwrap();
    let m = user_msg(conv.id, "find me", 0);
    let id = m.id;
    store.save_message(&m).await.unwrap();

    let got = store.get_message(id).await.unwrap();
    let got = got.expect("must find the saved message by id");
    assert_eq!(got.content, "find me");

    let missing = store.get_message(Uuid::new_v4()).await.unwrap();
    assert!(missing.is_none(), "unknown id returns None");
}

async fn last_messages_caps_to_limit(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("L")).await.unwrap();
    for i in 0..5 {
        store
            .save_message(&user_msg(conv.id, &format!("m{i}"), i))
            .await
            .unwrap();
    }
    let last3 = store.last_messages(conv.id, 3).await.unwrap();
    assert_eq!(last3.len(), 3);
    assert_eq!(last3[0].content, "m2");
    assert_eq!(last3[2].content, "m4");
}

async fn delete_conversation_removes_history(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("D")).await.unwrap();
    store
        .save_message(&user_msg(conv.id, "hello", 0))
        .await
        .unwrap();
    store.delete_conversation(conv.id).await.unwrap();
    let gone = store.get_conversation(conv.id).await.unwrap();
    assert!(gone.is_none(), "conversation is gone after delete");
    let history = store.load_history(conv.id).await.unwrap();
    assert!(history.is_empty(), "history is cascaded on delete");
}

async fn replace_history_overwrites(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("R")).await.unwrap();
    store
        .save_message(&user_msg(conv.id, "old", 0))
        .await
        .unwrap();
    let replacement = vec![
        user_msg(conv.id, "new1", 0),
        assistant_msg(conv.id, "new2", 0),
    ];
    store.replace_history(conv.id, &replacement).await.unwrap();
    let loaded = store.load_history(conv.id).await.unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].content, "new1");
    assert_eq!(loaded[1].content, "new2");
}

async fn sequential_saves_form_a_chain_ending_at_the_leaf(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Chain")).await.unwrap();
    let a = user_msg(conv.id, "a", 0);
    let b = assistant_msg(conv.id, "b", 1);
    let c = user_msg(conv.id, "c", 2);
    for m in [&a, &b, &c] {
        store.save_message(m).await.unwrap();
    }

    let history = store.load_history(conv.id).await.unwrap();
    let ids: Vec<Uuid> = history.iter().map(|m| m.id).collect();
    assert_eq!(ids, vec![a.id, b.id, c.id], "history follows the chain");
    assert_eq!(history[0].parent_id, None, "first message is a root");
    assert_eq!(history[1].parent_id, Some(a.id), "b hangs under a");
    assert_eq!(history[2].parent_id, Some(b.id), "c hangs under b");

    let last = store.last_messages(conv.id, 2).await.unwrap();
    let last_ids: Vec<Uuid> = last.iter().map(|m| m.id).collect();
    assert_eq!(last_ids, vec![b.id, c.id], "last_messages is the path tail");
}

async fn empty_conversation_loads_no_history(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Empty")).await.unwrap();
    assert!(store.load_history(conv.id).await.unwrap().is_empty());
    assert!(store.last_messages(conv.id, 5).await.unwrap().is_empty());
}

async fn long_chain_loads_in_order(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Long")).await.unwrap();
    let mut ids = Vec::new();
    for i in 0..1000 {
        let m = user_msg(conv.id, &format!("m{i}"), 0);
        ids.push(m.id);
        store.save_message(&m).await.unwrap();
    }
    let history = store.load_history(conv.id).await.unwrap();
    let loaded: Vec<Uuid> = history.iter().map(|m| m.id).collect();
    assert_eq!(loaded, ids, "1,000-row chain loads in save order");
}

async fn resaving_an_existing_id_keeps_the_leaf(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Idempotent")).await.unwrap();
    let a = user_msg(conv.id, "a", 0);
    let b = assistant_msg(conv.id, "b", 1);
    store.save_message(&a).await.unwrap();
    store.save_message(&b).await.unwrap();
    store.save_message(&a).await.unwrap();

    let c = user_msg(conv.id, "c", 2);
    store.save_message(&c).await.unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    let ids: Vec<Uuid> = history.iter().map(|m| m.id).collect();
    assert_eq!(
        ids,
        vec![a.id, b.id, c.id],
        "a re-save inserts nothing and does not move the leaf"
    );
    assert_eq!(history[2].parent_id, Some(b.id));
}

async fn deleting_a_long_chain_succeeds(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Big")).await.unwrap();
    for i in 0..1500 {
        store
            .save_message(&user_msg(conv.id, &format!("m{i}"), 0))
            .await
            .unwrap();
    }
    store.delete_conversation(conv.id).await.unwrap();
    assert!(store.get_conversation(conv.id).await.unwrap().is_none());
    assert!(store.load_history(conv.id).await.unwrap().is_empty());
}

async fn replace_history_rechains_the_new_rows(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Rechain")).await.unwrap();
    for i in 0..3 {
        store
            .save_message(&user_msg(conv.id, &format!("old{i}"), i))
            .await
            .unwrap();
    }
    let replacement = vec![
        user_msg(conv.id, "new1", 0),
        assistant_msg(conv.id, "new2", 0),
        user_msg(conv.id, "new3", 0),
    ];
    store.replace_history(conv.id, &replacement).await.unwrap();

    let loaded = store.load_history(conv.id).await.unwrap();
    let ids: Vec<Uuid> = loaded.iter().map(|m| m.id).collect();
    let expected: Vec<Uuid> = replacement.iter().map(|m| m.id).collect();
    assert_eq!(ids, expected, "history equals the re-inserted rows");
    assert_eq!(loaded[0].parent_id, None, "the first kept row is a root");
    assert_eq!(loaded[1].parent_id, Some(replacement[0].id));
    assert_eq!(loaded[2].parent_id, Some(replacement[1].id));
}

async fn tool_result_metadata_roundtrips(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Tool")).await.unwrap();
    store
        .save_message(&user_msg(conv.id, "run it", 0))
        .await
        .unwrap();
    let mut tool = Message::new(conv.id, MessageRole::Tool, "stdout:\nhi");
    tool.skill_name = Some("bash".into());
    tool.tool_call_id = Some("call_1".into());
    tool.tool_status = Some("error".into());
    tool.tool_data_json = Some(r#"{"kind":"terminal","exit_code":1}"#.into());
    store.save_message(&tool).await.unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    let loaded = &history[1];
    assert_eq!(loaded.tool_call_id.as_deref(), Some("call_1"));
    assert_eq!(loaded.tool_status.as_deref(), Some("error"));
    assert_eq!(
        loaded.tool_data_json.as_deref(),
        Some(r#"{"kind":"terminal","exit_code":1}"#)
    );

    let by_id = store.get_message(tool.id).await.unwrap().unwrap();
    assert_eq!(by_id.tool_call_id.as_deref(), Some("call_1"));
    assert_eq!(by_id.tool_status.as_deref(), Some("error"));
    assert!(by_id.tool_data_json.is_some());

    let plain = store.load_history(conv.id).await.unwrap();
    assert!(plain[0].tool_call_id.is_none() && plain[0].tool_data_json.is_none());
}

async fn traceparent_roundtrips(store: Box<dyn ConversationStore>) {
    const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let conv = store.create_conversation(Some("Trace")).await.unwrap();
    store
        .save_message(&user_msg(conv.id, "question", 0))
        .await
        .unwrap();
    let mut reply = Message::assistant(conv.id, "answer");
    reply.traceparent = Some(TRACEPARENT.into());
    store.save_message(&reply).await.unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(history[0].traceparent, None, "plain rows carry none");
    assert_eq!(history[1].traceparent.as_deref(), Some(TRACEPARENT));

    let by_id = store.get_message(reply.id).await.unwrap().unwrap();
    assert_eq!(by_id.traceparent.as_deref(), Some(TRACEPARENT));

    let children = store
        .children_of(conv.id, Some(history[0].id))
        .await
        .unwrap();
    assert_eq!(children[0].traceparent.as_deref(), Some(TRACEPARENT));

    let chain = store.ancestors(conv.id, reply.id).await.unwrap().unwrap();
    assert_eq!(chain[1].traceparent.as_deref(), Some(TRACEPARENT));
}

// -- Branching --

fn ids(messages: &[Message]) -> Vec<Uuid> {
    messages.iter().map(|m| m.id).collect()
}

/// A -> B -> C saved as a chain; returns the three messages.
async fn seed_chain(store: &dyn ConversationStore, conv: Uuid) -> [Message; 3] {
    let a = user_msg(conv, "a", 0);
    let b = assistant_msg(conv, "b", 1);
    let c = user_msg(conv, "c", 2);
    for m in [&a, &b, &c] {
        store.save_message(m).await.unwrap();
    }
    [a, b, c]
}

async fn branch_under_an_earlier_message_makes_a_sibling_leaf(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Branch")).await.unwrap();
    let [a, b, c] = seed_chain(store.as_ref(), conv.id).await;

    let d = assistant_msg(conv.id, "d", 1);
    store.branch_message(&d, Some(a.id)).await.unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(ids(&history), vec![a.id, d.id], "d is the leaf under a");
    assert_eq!(history[1].parent_id, Some(a.id));
    for kept in [&b, &c] {
        assert!(
            store.get_message(kept.id).await.unwrap().is_some(),
            "the old branch is kept"
        );
    }
}

async fn branch_with_no_parent_creates_a_second_root(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Roots")).await.unwrap();
    let [a, ..] = seed_chain(store.as_ref(), conv.id).await;

    let r = user_msg(conv.id, "r", 0);
    store.branch_message(&r, None).await.unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(ids(&history), vec![r.id], "the new root is the whole path");
    assert_eq!(history[0].parent_id, None);

    let siblings = store.list_siblings(conv.id, r.id).await.unwrap().unwrap();
    assert_eq!(ids(&siblings), vec![a.id, r.id], "roots are siblings");
}

async fn conflicting_branch_leaves_the_leaf_in_place(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Conflict")).await.unwrap();
    let [a, b, c] = seed_chain(store.as_ref(), conv.id).await;

    store.branch_message(&b, Some(a.id)).await.unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(ids(&history), vec![a.id, b.id, c.id], "leaf is still c");
}

async fn branch_under_a_foreign_parent_is_rejected(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Mine")).await.unwrap();
    let other = store.create_conversation(Some("Other")).await.unwrap();
    let [a, ..] = seed_chain(store.as_ref(), other.id).await;

    let m = user_msg(conv.id, "m", 0);
    let result = store.branch_message(&m, Some(a.id)).await;
    assert!(
        result.is_err(),
        "a parent in another conversation is refused"
    );
    assert!(store.get_message(m.id).await.unwrap().is_none());
}

async fn list_siblings_is_in_creation_order(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Siblings")).await.unwrap();
    let [a, b, _c] = seed_chain(store.as_ref(), conv.id).await;
    let b2 = assistant_msg(conv.id, "b2", 1);
    let b3 = assistant_msg(conv.id, "b3", 1);
    store.branch_message(&b2, Some(a.id)).await.unwrap();
    store.branch_message(&b3, Some(a.id)).await.unwrap();

    for probe in [b.id, b2.id, b3.id] {
        let siblings = store.list_siblings(conv.id, probe).await.unwrap().unwrap();
        assert_eq!(
            ids(&siblings),
            vec![b.id, b2.id, b3.id],
            "every sibling sees the same creation-ordered list"
        );
    }

    let only = store.list_siblings(conv.id, a.id).await.unwrap().unwrap();
    assert_eq!(ids(&only), vec![a.id], "an only child lists itself");
}

async fn children_of_lists_replies_in_creation_order(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Children")).await.unwrap();
    let [a, b, _c] = seed_chain(store.as_ref(), conv.id).await;
    let b2 = assistant_msg(conv.id, "b2", 1);
    store.branch_message(&b2, Some(a.id)).await.unwrap();

    let kids = store.children_of(conv.id, Some(a.id)).await.unwrap();
    assert_eq!(ids(&kids), vec![b.id, b2.id]);

    let roots = store.children_of(conv.id, None).await.unwrap();
    assert_eq!(ids(&roots), vec![a.id], "None lists the roots");

    let leaf_kids = store.children_of(conv.id, Some(b2.id)).await.unwrap();
    assert!(leaf_kids.is_empty());
}

async fn ancestors_run_root_first(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Ancestors")).await.unwrap();
    let [a, b, c] = seed_chain(store.as_ref(), conv.id).await;

    let chain = store.ancestors(conv.id, c.id).await.unwrap().unwrap();
    assert_eq!(ids(&chain), vec![a.id, b.id, c.id], "root first, self last");

    let root = store.ancestors(conv.id, a.id).await.unwrap().unwrap();
    assert_eq!(ids(&root), vec![a.id]);
}

async fn switch_to_descends_through_the_most_recent_child(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Switch")).await.unwrap();
    // a -> b -> c, plus a -> b2 -> c2 -> c3 and a -> b3 (newest child of a).
    let [a, b, c] = seed_chain(store.as_ref(), conv.id).await;
    let b2 = assistant_msg(conv.id, "b2", 1);
    let c2 = user_msg(conv.id, "c2", 2);
    let c3 = assistant_msg(conv.id, "c3", 3);
    store.branch_message(&b2, Some(a.id)).await.unwrap();
    store.save_message(&c2).await.unwrap();
    store.save_message(&c3).await.unwrap();
    let b3 = assistant_msg(conv.id, "b3", 1);
    store.branch_message(&b3, Some(a.id)).await.unwrap();

    let leaf = store.switch_to(conv.id, b2.id).await.unwrap();
    assert_eq!(leaf, Some(c3.id), "descends b2 -> c2 -> c3");
    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(ids(&history), vec![a.id, b2.id, c2.id, c3.id]);

    let leaf = store.switch_to(conv.id, b.id).await.unwrap();
    assert_eq!(leaf, Some(c.id), "descends b -> c");

    let leaf = store.switch_to(conv.id, a.id).await.unwrap();
    assert_eq!(leaf, Some(b3.id), "the newest child of a wins");
    assert_eq!(store.load_history(conv.id).await.unwrap().len(), 2);
}

async fn switch_to_keeps_updated_at(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Quiet")).await.unwrap();
    let [a, b, _c] = seed_chain(store.as_ref(), conv.id).await;
    store
        .branch_message(&assistant_msg(conv.id, "b2", 1), Some(a.id))
        .await
        .unwrap();
    let before = store.get_conversation(conv.id).await.unwrap().unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    store.switch_to(conv.id, b.id).await.unwrap();

    let after = store.get_conversation(conv.id).await.unwrap().unwrap();
    assert_eq!(
        after.updated_at, before.updated_at,
        "a switch is not an edit"
    );
}

async fn set_leaf_sets_the_leaf_exactly(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Leaf")).await.unwrap();
    let [a, b, _c] = seed_chain(store.as_ref(), conv.id).await;

    let set = store.set_leaf(conv.id, b.id).await.unwrap();
    assert_eq!(set, Some(b.id));
    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(ids(&history), vec![a.id, b.id], "no descent past b");
}

async fn foreign_conversation_messages_return_none(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Mine")).await.unwrap();
    let other = store.create_conversation(Some("Other")).await.unwrap();
    let [a, ..] = seed_chain(store.as_ref(), other.id).await;
    let unknown = Uuid::new_v4();

    for probe in [a.id, unknown] {
        assert!(store.list_siblings(conv.id, probe).await.unwrap().is_none());
        assert!(store.ancestors(conv.id, probe).await.unwrap().is_none());
        assert!(store.switch_to(conv.id, probe).await.unwrap().is_none());
        assert!(store.set_leaf(conv.id, probe).await.unwrap().is_none());
    }
    let theirs = store.load_history(other.id).await.unwrap();
    assert_eq!(theirs.len(), 3, "the other conversation is untouched");
}

async fn delete_run_rows_removes_a_runs_rows_but_keeps_the_anchor(
    store: Box<dyn ConversationStore>,
) {
    let conv = store.create_conversation(Some("Runs")).await.unwrap();
    let run = Uuid::new_v4();
    let mut user = user_msg(conv.id, "q", 0);
    user.run_id = Some(run);
    let mut partial1 = assistant_msg(conv.id, "thinking", 1);
    partial1.run_id = Some(run);
    let mut partial2 = assistant_msg(conv.id, "more", 2);
    partial2.run_id = Some(run);
    let mut other_run = assistant_msg(conv.id, "other", 1);
    other_run.run_id = Some(Uuid::new_v4());
    store.save_message(&user).await.unwrap();
    store
        .branch_message(&other_run, Some(user.id))
        .await
        .unwrap();
    store
        .branch_message(&partial1, Some(user.id))
        .await
        .unwrap();
    store.save_message(&partial2).await.unwrap();

    let removed = store
        .delete_run_rows(conv.id, run, Some(user.id))
        .await
        .unwrap();
    assert_eq!(removed, 2, "both partial rows go");
    assert!(store.get_message(user.id).await.unwrap().is_some());
    assert!(store.get_message(other_run.id).await.unwrap().is_some());
    assert!(store.get_message(partial1.id).await.unwrap().is_none());
    assert!(store.get_message(partial2.id).await.unwrap().is_none());

    store.set_leaf(conv.id, user.id).await.unwrap();
    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(ids(&history), vec![user.id]);
}

async fn delete_run_rows_without_keep_removes_every_row_of_the_run(
    store: Box<dyn ConversationStore>,
) {
    let conv = store.create_conversation(Some("Runs")).await.unwrap();
    let run = Uuid::new_v4();
    let mut a = user_msg(conv.id, "a", 0);
    a.run_id = Some(run);
    let mut b = assistant_msg(conv.id, "b", 1);
    b.run_id = Some(run);
    store.save_message(&a).await.unwrap();
    store.save_message(&b).await.unwrap();

    let removed = store.delete_run_rows(conv.id, run, None).await.unwrap();
    assert_eq!(removed, 2);
    assert!(store.get_message(a.id).await.unwrap().is_none());
}

async fn delete_run_rows_fails_when_another_row_hangs_under_a_deleted_one(
    store: Box<dyn ConversationStore>,
) {
    let conv = store.create_conversation(Some("Runs")).await.unwrap();
    let run = Uuid::new_v4();
    let user = user_msg(conv.id, "q", 0);
    let mut partial = assistant_msg(conv.id, "partial", 1);
    partial.run_id = Some(run);
    let late = user_msg(conv.id, "late", 2);
    store.save_message(&user).await.unwrap();
    store.save_message(&partial).await.unwrap();
    store.save_message(&late).await.unwrap();

    let result = store.delete_run_rows(conv.id, run, Some(user.id)).await;
    assert!(
        result.is_err(),
        "a child of a deleted row blocks the delete"
    );
    assert!(
        store.get_message(partial.id).await.unwrap().is_some(),
        "nothing was deleted"
    );
}

async fn finish_reason_round_trips(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("t")).await.unwrap();
    let user = user_msg(conv.id, "hi", 0);
    let mut stopped = assistant_msg(conv.id, "partial", 1);
    stopped.finish_reason = Some(MessageFinishReason::Cancelled);
    let plain = assistant_msg(conv.id, "full", 2);
    for m in [&user, &stopped, &plain] {
        store.save_message(m).await.unwrap();
    }

    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(history[0].finish_reason, None, "user message has none");
    assert_eq!(
        history[1].finish_reason,
        Some(MessageFinishReason::Cancelled),
        "load_history keeps the finish reason"
    );
    assert_eq!(history[2].finish_reason, None, "unmarked message has none");

    let tail = store.last_messages(conv.id, 2).await.unwrap();
    assert_eq!(
        tail[0].finish_reason,
        Some(MessageFinishReason::Cancelled),
        "last_messages keeps the finish reason"
    );
    let single = store.get_message(stopped.id).await.unwrap().unwrap();
    assert_eq!(single.finish_reason, Some(MessageFinishReason::Cancelled));
}

async fn replace_history_keeps_finish_reason(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("t")).await.unwrap();
    let mut stopped = assistant_msg(conv.id, "partial", 0);
    stopped.finish_reason = Some(MessageFinishReason::Cancelled);
    store
        .replace_history(conv.id, std::slice::from_ref(&stopped))
        .await
        .unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(
        history[0].finish_reason,
        Some(MessageFinishReason::Cancelled),
        "replace_history preserves the finish reason"
    );
}

macro_rules! contract_tests {
    ($store_name:ident, $constructor:expr) => {
        mod $store_name {
            use super::*;

            #[tokio::test]
            async fn t_finish_reason_round_trips() {
                finish_reason_round_trips($constructor).await;
            }

            #[tokio::test]
            async fn t_replace_history_keeps_finish_reason() {
                replace_history_keeps_finish_reason($constructor).await;
            }

            #[tokio::test]
            async fn t_create_and_get_roundtrip() {
                create_and_get_roundtrip($constructor).await;
            }

            #[tokio::test]
            async fn t_create_untitled_does_not_lock() {
                create_untitled_does_not_lock($constructor).await;
            }

            #[tokio::test]
            async fn t_create_with_id_idempotent() {
                create_with_id_idempotent($constructor).await;
            }

            #[tokio::test]
            async fn t_list_orders_by_recency() {
                list_orders_by_recency($constructor).await;
            }

            #[tokio::test]
            async fn t_update_title_locks() {
                update_title_locks($constructor).await;
            }

            #[tokio::test]
            async fn t_update_title_does_not_bump_updated_at() {
                update_title_does_not_bump_updated_at($constructor).await;
            }

            #[tokio::test]
            async fn t_update_metadata_title_and_pin() {
                update_metadata_title_and_pin($constructor).await;
            }

            #[tokio::test]
            async fn t_update_metadata_repin_keeps_pinned_at() {
                update_metadata_repin_keeps_pinned_at($constructor).await;
            }

            #[tokio::test]
            async fn t_update_metadata_unpin_clears_pinned_at() {
                update_metadata_unpin_clears_pinned_at($constructor).await;
            }

            #[tokio::test]
            async fn t_update_metadata_unknown_id_is_not_found() {
                update_metadata_unknown_id_is_not_found($constructor).await;
            }

            #[tokio::test]
            async fn t_update_metadata_overwrites_locked_title() {
                update_metadata_overwrites_locked_title($constructor).await;
            }

            #[tokio::test]
            async fn t_list_orders_pinned_first_newest_pin_first() {
                list_orders_pinned_first_newest_pin_first($constructor).await;
            }

            #[tokio::test]
            async fn t_set_generated_title_writes_unlocked_row() {
                set_generated_title_writes_unlocked_row($constructor).await;
            }

            #[tokio::test]
            async fn t_set_generated_title_skips_locked_row() {
                set_generated_title_skips_locked_row($constructor).await;
            }

            #[tokio::test]
            async fn t_set_generated_title_skips_missing_row() {
                set_generated_title_skips_missing_row($constructor).await;
            }

            #[tokio::test]
            async fn t_save_and_load_history() {
                save_and_load_history($constructor).await;
            }

            #[tokio::test]
            async fn t_get_message_by_id() {
                get_message_by_id($constructor).await;
            }

            #[tokio::test]
            async fn t_last_messages_caps_to_limit() {
                last_messages_caps_to_limit($constructor).await;
            }

            #[tokio::test]
            async fn t_delete_conversation_removes_history() {
                delete_conversation_removes_history($constructor).await;
            }

            #[tokio::test]
            async fn t_replace_history_overwrites() {
                replace_history_overwrites($constructor).await;
            }

            #[tokio::test]
            async fn t_sequential_saves_form_a_chain_ending_at_the_leaf() {
                sequential_saves_form_a_chain_ending_at_the_leaf($constructor).await;
            }

            #[tokio::test]
            async fn t_empty_conversation_loads_no_history() {
                empty_conversation_loads_no_history($constructor).await;
            }

            #[tokio::test]
            async fn t_long_chain_loads_in_order() {
                long_chain_loads_in_order($constructor).await;
            }

            #[tokio::test]
            async fn t_resaving_an_existing_id_keeps_the_leaf() {
                resaving_an_existing_id_keeps_the_leaf($constructor).await;
            }

            #[tokio::test]
            async fn t_deleting_a_long_chain_succeeds() {
                deleting_a_long_chain_succeeds($constructor).await;
            }

            #[tokio::test]
            async fn t_replace_history_rechains_the_new_rows() {
                replace_history_rechains_the_new_rows($constructor).await;
            }

            #[tokio::test]
            async fn t_tool_result_metadata_roundtrips() {
                tool_result_metadata_roundtrips($constructor).await;
            }

            #[tokio::test]
            async fn t_traceparent_roundtrips() {
                traceparent_roundtrips($constructor).await;
            }

            #[tokio::test]
            async fn t_branch_under_an_earlier_message_makes_a_sibling_leaf() {
                branch_under_an_earlier_message_makes_a_sibling_leaf($constructor).await;
            }

            #[tokio::test]
            async fn t_branch_with_no_parent_creates_a_second_root() {
                branch_with_no_parent_creates_a_second_root($constructor).await;
            }

            #[tokio::test]
            async fn t_conflicting_branch_leaves_the_leaf_in_place() {
                conflicting_branch_leaves_the_leaf_in_place($constructor).await;
            }

            #[tokio::test]
            async fn t_branch_under_a_foreign_parent_is_rejected() {
                branch_under_a_foreign_parent_is_rejected($constructor).await;
            }

            #[tokio::test]
            async fn t_list_siblings_is_in_creation_order() {
                list_siblings_is_in_creation_order($constructor).await;
            }

            #[tokio::test]
            async fn t_children_of_lists_replies_in_creation_order() {
                children_of_lists_replies_in_creation_order($constructor).await;
            }

            #[tokio::test]
            async fn t_ancestors_run_root_first() {
                ancestors_run_root_first($constructor).await;
            }

            #[tokio::test]
            async fn t_switch_to_descends_through_the_most_recent_child() {
                switch_to_descends_through_the_most_recent_child($constructor).await;
            }

            #[tokio::test]
            async fn t_switch_to_keeps_updated_at() {
                switch_to_keeps_updated_at($constructor).await;
            }

            #[tokio::test]
            async fn t_set_leaf_sets_the_leaf_exactly() {
                set_leaf_sets_the_leaf_exactly($constructor).await;
            }

            #[tokio::test]
            async fn t_foreign_conversation_messages_return_none() {
                foreign_conversation_messages_return_none($constructor).await;
            }

            #[tokio::test]
            async fn t_delete_run_rows_removes_a_runs_rows_but_keeps_the_anchor() {
                delete_run_rows_removes_a_runs_rows_but_keeps_the_anchor($constructor).await;
            }

            #[tokio::test]
            async fn t_delete_run_rows_without_keep_removes_every_row_of_the_run() {
                delete_run_rows_without_keep_removes_every_row_of_the_run($constructor).await;
            }

            #[tokio::test]
            async fn t_delete_run_rows_fails_when_another_row_hangs_under_a_deleted_one() {
                delete_run_rows_fails_when_another_row_hangs_under_a_deleted_one($constructor)
                    .await;
            }
        }
    };
}

contract_tests!(sqlite, sqlite_store().await);
contract_tests!(memory, memory_store());
