//! [`TurnStore`] — the conversation store one turn saves its rows through.
//!
//! It stamps every row with the turn's run id, and can hold a branch parent
//! for the turn's first row. A regenerate arms it with the user message `U`,
//! so the reply's first row becomes a child of `U` and the rest append.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use assistant_core::types::conversation::Message;
use assistant_storage::conversations::{
    ConversationPatch, ConversationRecord, ConversationSearchHit, ConversationStore, MetadataUpdate,
};

/// Wraps a [`ConversationStore`] for the duration of one turn.
pub(crate) struct TurnStore {
    inner: Arc<dyn ConversationStore>,
    run_id: Option<Uuid>,
    pending_parent: Mutex<Option<Uuid>>,
}

impl TurnStore {
    /// Wrap `inner`. Rows saved without a run id are stamped with `run_id`.
    pub(crate) fn new(inner: impl ConversationStore + 'static, run_id: Option<Uuid>) -> Self {
        Self {
            inner: Arc::new(inner),
            run_id,
            pending_parent: Mutex::new(None),
        }
    }

    /// Make the next [`save_message`](ConversationStore::save_message) a
    /// branch under `parent`.
    pub(crate) fn arm_branch(&self, parent: Uuid) {
        self.set_pending(Some(parent));
    }

    /// Whether a branch parent is still waiting for the turn's first row,
    /// that is, whether the turn has persisted nothing yet.
    pub(crate) fn branch_pending(&self) -> bool {
        self.pending_parent.lock().is_ok_and(|p| p.is_some())
    }

    fn set_pending(&self, parent: Option<Uuid>) {
        if let Ok(mut pending) = self.pending_parent.lock() {
            *pending = parent;
        }
    }

    fn take_pending(&self) -> Option<Uuid> {
        self.pending_parent
            .lock()
            .ok()
            .and_then(|mut pending| pending.take())
    }

    fn stamped(&self, msg: &Message) -> Message {
        let mut stamped = msg.clone();
        if stamped.run_id.is_none() {
            stamped.run_id = self.run_id;
        }
        stamped
    }
}

#[async_trait]
impl ConversationStore for TurnStore {
    async fn save_message(&self, msg: &Message) -> Result<()> {
        let stamped = self.stamped(msg);
        let Some(parent) = self.take_pending() else {
            return self.inner.save_message(&stamped).await;
        };
        let saved = self.inner.branch_message(&stamped, Some(parent)).await;
        if saved.is_err() {
            // Nothing was written: keep the branch for the next row.
            self.set_pending(Some(parent));
        }
        saved
    }

    async fn branch_message(&self, msg: &Message, parent_id: Option<Uuid>) -> Result<()> {
        self.inner
            .branch_message(&self.stamped(msg), parent_id)
            .await
    }

    async fn create_conversation_with_id(
        &self,
        id: Uuid,
        title: Option<&str>,
    ) -> Result<ConversationRecord> {
        self.inner.create_conversation_with_id(id, title).await
    }

    async fn create_conversation(&self, title: Option<&str>) -> Result<ConversationRecord> {
        self.inner.create_conversation(title).await
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<ConversationRecord>> {
        self.inner.get_conversation(id).await
    }

    async fn list_conversations(&self) -> Result<Vec<ConversationRecord>> {
        self.inner.list_conversations().await
    }

    async fn search_conversations(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ConversationSearchHit>> {
        self.inner.search_conversations(query, limit).await
    }

    async fn update_title(&self, id: Uuid, title: &str) -> Result<()> {
        self.inner.update_title(id, title).await
    }

    async fn update_metadata(&self, id: Uuid, patch: ConversationPatch) -> Result<MetadataUpdate> {
        self.inner.update_metadata(id, patch).await
    }

    async fn set_generated_title(&self, id: Uuid, title: &str) -> Result<bool> {
        self.inner.set_generated_title(id, title).await
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<()> {
        self.inner.delete_conversation(id).await
    }

    async fn load_history(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        self.inner.load_history(conversation_id).await
    }

    async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>> {
        self.inner.get_message(message_id).await
    }

    async fn last_messages(&self, conversation_id: Uuid, limit: i64) -> Result<Vec<Message>> {
        self.inner.last_messages(conversation_id, limit).await
    }

    async fn children_of(
        &self,
        conversation_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<Message>> {
        self.inner.children_of(conversation_id, parent_id).await
    }

    async fn list_siblings(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Vec<Message>>> {
        self.inner.list_siblings(conversation_id, message_id).await
    }

    async fn ancestors(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Vec<Message>>> {
        self.inner.ancestors(conversation_id, message_id).await
    }

    async fn switch_to(&self, conversation_id: Uuid, message_id: Uuid) -> Result<Option<Uuid>> {
        self.inner.switch_to(conversation_id, message_id).await
    }

    async fn set_leaf(&self, conversation_id: Uuid, message_id: Uuid) -> Result<Option<Uuid>> {
        self.inner.set_leaf(conversation_id, message_id).await
    }

    async fn delete_run_rows(
        &self,
        conversation_id: Uuid,
        run_id: Uuid,
        keep: Option<Uuid>,
    ) -> Result<u64> {
        self.inner
            .delete_run_rows(conversation_id, run_id, keep)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::conversation::MessageRole;
    use assistant_storage::conversations::InMemoryConversationStore;

    async fn store(run_id: Option<Uuid>) -> (TurnStore, Uuid) {
        let store = TurnStore::new(InMemoryConversationStore::new(), run_id);
        let conv = Uuid::new_v4();
        store.create_conversation_with_id(conv, None).await.unwrap();
        (store, conv)
    }

    #[tokio::test]
    async fn rows_are_stamped_with_the_run_id() {
        let run_id = Uuid::new_v4();
        let (store, conv) = store(Some(run_id)).await;
        let row = Message::new(conv, MessageRole::User, "hi");
        store.save_message(&row).await.unwrap();
        let saved = store.get_message(row.id).await.unwrap().unwrap();
        assert_eq!(saved.run_id, Some(run_id));
    }

    #[tokio::test]
    async fn an_explicit_run_id_is_kept() {
        let (store, conv) = store(Some(Uuid::new_v4())).await;
        let own = Uuid::new_v4();
        let mut row = Message::new(conv, MessageRole::User, "hi");
        row.run_id = Some(own);
        store.save_message(&row).await.unwrap();
        let saved = store.get_message(row.id).await.unwrap().unwrap();
        assert_eq!(saved.run_id, Some(own));
    }

    #[tokio::test]
    async fn the_armed_parent_applies_to_the_first_row_only() {
        let (store, conv) = store(None).await;
        let user = Message::new(conv, MessageRole::User, "U");
        let old_reply = Message::new(conv, MessageRole::Assistant, "old");
        store.save_message(&user).await.unwrap();
        store.save_message(&old_reply).await.unwrap();

        store.arm_branch(user.id);
        assert!(store.branch_pending());
        let first = Message::new(conv, MessageRole::Assistant, "new");
        let second = Message::new(conv, MessageRole::Assistant, "newer");
        store.save_message(&first).await.unwrap();
        assert!(!store.branch_pending());
        store.save_message(&second).await.unwrap();

        let children = store.children_of(conv, Some(user.id)).await.unwrap();
        assert_eq!(children.len(), 2, "old and new reply under U");
        let first = store.get_message(first.id).await.unwrap().unwrap();
        let second = store.get_message(second.id).await.unwrap().unwrap();
        assert_eq!(first.parent_id, Some(user.id));
        assert_eq!(second.parent_id, Some(first.id), "later rows append");
    }

    #[tokio::test]
    async fn a_failed_branch_save_keeps_the_parent_armed() {
        let (store, conv) = store(None).await;
        store.arm_branch(Uuid::new_v4());
        let row = Message::new(conv, MessageRole::Assistant, "x");
        assert!(store.save_message(&row).await.is_err());
        assert!(store.branch_pending(), "the next row retries the branch");
    }
}
