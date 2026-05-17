//! Contract tests for the [`AttachmentStore`] trait.

use assistant_core::AttachmentMeta;
use assistant_storage::{AttachmentStore, InMemoryAttachmentStore};
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
