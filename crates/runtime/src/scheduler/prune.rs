//! Periodic conversation-event pruning — drops rows that have aged past
//! their configured TTL.

use assistant_storage::ConversationEventStore as _;
use tracing::{info, warn};

use assistant_storage::StorageLayer;

/// Delete expired conversation events. Logs the result at debug level.
pub(crate) async fn prune_conversation_events(storage: &StorageLayer) {
    let store = storage.conversation_event_store();
    match store.prune_expired().await {
        Ok(n) if n > 0 => info!("Pruned {n} expired conversation event(s)"),
        Ok(_) => {}
        Err(e) => warn!("Failed to prune conversation events: {e}"),
    }
}
