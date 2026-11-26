//! Contract tests for the [`AttachmentStore`] trait.

use std::time::Duration;

use assistant_core::types::conversation::Message;
use assistant_core::{AttachmentMeta, agent_base_dir};
use assistant_storage::{
    AttachmentStore, ConversationStore, InMemoryAttachmentStore, StorageLayer,
};
use chrono::Utc;
use uuid::Uuid;

fn make_meta(conv_id: Uuid, agent_id: &str, filename: &str) -> AttachmentMeta {
    AttachmentMeta {
        id: Uuid::new_v4(),
        message_id: None,
        conversation_id: conv_id,
        agent_id: agent_id.to_string(),
        filename: filename.to_string(),
        mime_type: "image/png".to_string(),
        size_bytes: 4,
        created_at: Utc::now(),
    }
}

// SQLite scenarios live in attachments.rs's #[cfg(test)] module already,
// so this file focuses on the InMemory impl — it ensures the InMemory
// store's surface honours the trait contract.

#[tokio::test]
async fn memory_store_and_get_meta() {
    let store = InMemoryAttachmentStore::new();
    let conv = Uuid::new_v4();
    let meta = make_meta(conv, "default", "test.png");
    let bytes: &[u8] = &[1, 2, 3, 4];
    store.store(&meta, bytes).await.unwrap();
    let got = store.get_meta(meta.id).await.unwrap();
    assert_eq!(got.filename, "test.png");
}

#[tokio::test]
async fn memory_load_bytes_roundtrip() {
    let store = InMemoryAttachmentStore::new();
    let conv = Uuid::new_v4();
    let meta = make_meta(conv, "default", "data.bin");
    let bytes: &[u8] = b"hello world";
    store.store(&meta, bytes).await.unwrap();
    let loaded = store.load_bytes(meta.id).await.unwrap();
    assert_eq!(loaded.as_slice(), bytes);
}

#[tokio::test]
async fn memory_list_for_conversation() {
    let store = InMemoryAttachmentStore::new();
    let conv = Uuid::new_v4();
    let other_conv = Uuid::new_v4();
    let m1 = make_meta(conv, "default", "a.png");
    let m2 = make_meta(conv, "default", "b.png");
    let m3 = make_meta(other_conv, "default", "c.png");
    let bytes: &[u8] = &[];
    store.store(&m1, bytes).await.unwrap();
    store.store(&m2, bytes).await.unwrap();
    store.store(&m3, bytes).await.unwrap();
    let listed = store.list_for_conversation(conv).await.unwrap();
    assert_eq!(listed.len(), 2);
}

#[tokio::test]
async fn memory_link_to_message_updates_meta() {
    let store = InMemoryAttachmentStore::new();
    let conv = Uuid::new_v4();
    let meta = make_meta(conv, "default", "linkme.png");
    let bytes: &[u8] = &[];
    store.store(&meta, bytes).await.unwrap();
    let msg = Uuid::new_v4();
    store.link_to_message(meta.id, msg).await.unwrap();
    let got = store.get_meta(meta.id).await.unwrap();
    assert_eq!(got.message_id, Some(msg));
}

#[tokio::test]
async fn memory_list_for_message_after_link() {
    let store = InMemoryAttachmentStore::new();
    let conv = Uuid::new_v4();
    let msg = Uuid::new_v4();
    let m1 = make_meta(conv, "default", "a.png");
    let m2 = make_meta(conv, "default", "b.png");
    let bytes: &[u8] = &[];
    store.store(&m1, bytes).await.unwrap();
    store.store(&m2, bytes).await.unwrap();
    store.link_to_message(m1.id, msg).await.unwrap();
    let listed = store.list_for_message(msg).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].filename, "a.png");
}

// -- carry_forward: shared attachments across message versions ---------------

/// A store plus a conversation with an original message `from` and its
/// edited sibling `to`.
struct Env {
    store: Box<dyn AttachmentStore>,
    conv: Uuid,
    from: Uuid,
    to: Uuid,
    /// Per-test agent id, so attachment bytes land in a directory of our own.
    agent_id: String,
    /// Kept alive so the SQLite pool outlives the store.
    storage: Option<StorageLayer>,
}

impl Drop for Env {
    fn drop(&mut self) {
        // The SQLite store writes bytes under `~/.assistant/agents/{agent_id}`.
        let _ = std::fs::remove_dir_all(agent_base_dir(&self.agent_id));
    }
}

fn unique_agent() -> String {
    format!("contract-test-{}", Uuid::new_v4())
}

async fn sqlite_env() -> Env {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let conversations = storage.conversation_store();
    let conv = conversations
        .create_conversation(Some("t"))
        .await
        .unwrap()
        .id;
    let from = Message::user(conv, "original");
    let to = Message::user(conv, "edited");
    conversations.save_message(&from).await.unwrap();
    conversations.save_message(&to).await.unwrap();
    Env {
        store: Box::new(storage.attachment_store()),
        conv,
        from: from.id,
        to: to.id,
        agent_id: unique_agent(),
        storage: Some(storage),
    }
}

fn memory_env() -> Env {
    Env {
        store: Box::new(InMemoryAttachmentStore::new()),
        conv: Uuid::new_v4(),
        from: Uuid::new_v4(),
        to: Uuid::new_v4(),
        agent_id: unique_agent(),
        storage: None,
    }
}

/// Store an attachment bound directly to `message`, `age_ms` after a fixed base
/// time so creation order is deterministic.
async fn attach(env: &Env, message: Uuid, filename: &str, age_ms: u64) -> AttachmentMeta {
    let mut meta = make_meta(env.conv, &env.agent_id, filename);
    meta.message_id = Some(message);
    meta.created_at = chrono::DateTime::UNIX_EPOCH + Duration::from_millis(age_ms);
    env.store.store(&meta, filename.as_bytes()).await.unwrap();
    meta
}

fn names(list: &[AttachmentMeta]) -> Vec<&str> {
    list.iter().map(|m| m.filename.as_str()).collect()
}

async fn carry_forward_shares_and_keeps_original(env: Env) {
    let x = attach(&env, env.from, "x.png", 1).await;
    attach(&env, env.from, "y.png", 2).await;

    env.store.carry_forward(env.from, env.to).await.unwrap();

    let carried = env.store.list_for_message(env.to).await.unwrap();
    assert_eq!(
        names(&carried),
        ["x.png", "y.png"],
        "target lists the source's attachments"
    );
    let original = env.store.list_for_message(env.from).await.unwrap();
    assert_eq!(names(&original), ["x.png", "y.png"], "source is unchanged");
    assert_eq!(
        original[0].message_id,
        Some(env.from),
        "the attachment still belongs to its original message"
    );
    assert_eq!(
        carried[0].id, x.id,
        "the carried row is the same attachment"
    );
}

async fn carry_forward_twice_links_once(env: Env) {
    attach(&env, env.from, "x.png", 1).await;

    env.store.carry_forward(env.from, env.to).await.unwrap();
    env.store.carry_forward(env.from, env.to).await.unwrap();

    let carried = env.store.list_for_message(env.to).await.unwrap();
    assert_eq!(
        names(&carried),
        ["x.png"],
        "a repeated carry lists each attachment once"
    );
}

async fn list_merges_direct_and_linked_in_creation_order(env: Env) {
    attach(&env, env.from, "old.png", 1).await;
    attach(&env, env.to, "direct.png", 2).await;
    attach(&env, env.from, "new.png", 3).await;

    env.store.carry_forward(env.from, env.to).await.unwrap();

    let listed = env.store.list_for_message(env.to).await.unwrap();
    assert_eq!(
        names(&listed),
        ["old.png", "direct.png", "new.png"],
        "direct and linked attachments interleave by creation time"
    );
}

async fn carried_attachment_bytes_and_meta_still_load(env: Env) {
    let x = attach(&env, env.from, "x.png", 1).await;
    env.store.carry_forward(env.from, env.to).await.unwrap();

    let carried = env.store.list_for_message(env.to).await.unwrap();
    let got = env.store.get_meta(carried[0].id).await.unwrap();
    assert_eq!(got.filename, "x.png");
    let bytes = env.store.load_bytes(x.id).await.unwrap();
    assert_eq!(bytes.as_slice(), b"x.png", "bytes are shared, not copied");
}

async fn carry_forward_without_attachments_is_a_no_op(env: Env) {
    env.store.carry_forward(env.from, env.to).await.unwrap();
    assert!(env.store.list_for_message(env.to).await.unwrap().is_empty());
}

#[tokio::test]
async fn sqlite_carry_forward_shares_and_keeps_original() {
    carry_forward_shares_and_keeps_original(sqlite_env().await).await;
}

#[tokio::test]
async fn memory_carry_forward_shares_and_keeps_original() {
    carry_forward_shares_and_keeps_original(memory_env()).await;
}

#[tokio::test]
async fn sqlite_carry_forward_twice_links_once() {
    carry_forward_twice_links_once(sqlite_env().await).await;
}

#[tokio::test]
async fn memory_carry_forward_twice_links_once() {
    carry_forward_twice_links_once(memory_env()).await;
}

#[tokio::test]
async fn sqlite_list_merges_direct_and_linked_in_creation_order() {
    list_merges_direct_and_linked_in_creation_order(sqlite_env().await).await;
}

#[tokio::test]
async fn memory_list_merges_direct_and_linked_in_creation_order() {
    list_merges_direct_and_linked_in_creation_order(memory_env()).await;
}

#[tokio::test]
async fn sqlite_carried_attachment_bytes_and_meta_still_load() {
    carried_attachment_bytes_and_meta_still_load(sqlite_env().await).await;
}

#[tokio::test]
async fn memory_carried_attachment_bytes_and_meta_still_load() {
    carried_attachment_bytes_and_meta_still_load(memory_env()).await;
}

#[tokio::test]
async fn sqlite_carry_forward_without_attachments_is_a_no_op() {
    carry_forward_without_attachments_is_a_no_op(sqlite_env().await).await;
}

#[tokio::test]
async fn memory_carry_forward_without_attachments_is_a_no_op() {
    carry_forward_without_attachments_is_a_no_op(memory_env()).await;
}

/// SQLite only: the in-memory store cannot see conversations.
#[tokio::test]
async fn sqlite_deleting_the_conversation_removes_the_links() {
    let env = sqlite_env().await;
    // Unbound attachment (as an upload that is later linked by id) so only
    // the link table points at the messages.
    let meta = make_meta(env.conv, &env.agent_id, "x.png");
    env.store.store(&meta, b"x").await.unwrap();
    let storage = env.storage.as_ref().unwrap();
    sqlx::query("INSERT INTO message_attachment_links (message_id, attachment_id) VALUES (?, ?)")
        .bind(env.to.to_string())
        .bind(meta.id.to_string())
        .execute(&storage.pool)
        .await
        .unwrap();

    storage
        .conversation_store()
        .delete_conversation(env.conv)
        .await
        .unwrap();

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message_attachment_links")
        .fetch_one(&storage.pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0, "links go with their conversation");
}
