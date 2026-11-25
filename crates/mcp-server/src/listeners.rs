//! Fan-out of list-changed notifications to every connected session.
//!
//! `2026-07-28` sessions receive them only on the `subscriptions/listen`
//! streams they opened; `2025-11-25` sessions, which negotiated through
//! `initialize`, receive the classic notifications.

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::RoleServer;
use rmcp::service::{Peer, SubscriptionContext, SubscriptionSink};
use tokio::sync::Mutex;
use tracing::debug;

/// The sessions to tell when installed skills change.
///
/// Clones share one registry. Whoever builds servers owns it and passes it to
/// each one, so a change made through one server reaches the sessions of all
/// of them.
#[derive(Clone, Default)]
pub struct ListChangeListeners {
    inner: Arc<Mutex<Registry>>,
}

#[derive(Default)]
struct Registry {
    next_id: u64,
    subscriptions: HashMap<u64, SubscriptionSink>,
    legacy_peers: Vec<Peer<RoleServer>>,
}

impl Registry {
    fn drop_closed_peers(&mut self) {
        self.legacy_peers.retain(|peer| !peer.is_transport_closed());
    }
}

impl ListChangeListeners {
    /// Keeps `subscription` registered until the client cancels it or
    /// disconnects.
    pub async fn listen(&self, subscription: SubscriptionContext) {
        let id = {
            let mut registry = self.inner.lock().await;
            registry.next_id += 1;
            let id = registry.next_id;
            registry
                .subscriptions
                .insert(id, subscription.sink().clone());
            id
        };
        subscription.cancelled().await;
        self.inner.lock().await.subscriptions.remove(&id);
    }

    /// Registers a session that negotiated a revision with classic
    /// list-changed notifications.
    pub async fn add_legacy_peer(&self, peer: Peer<RoleServer>) {
        let mut registry = self.inner.lock().await;
        registry.drop_closed_peers();
        registry.legacy_peers.push(peer);
    }

    /// Tells every listening session that tools, prompts and resources changed.
    pub async fn notify_all_changed(&self) {
        let (subscriptions, legacy_peers) = {
            let mut registry = self.inner.lock().await;
            registry.drop_closed_peers();
            (
                registry.subscriptions.values().cloned().collect::<Vec<_>>(),
                registry.legacy_peers.clone(),
            )
        };

        for sink in subscriptions {
            let accepted = sink.accepted();
            if accepted.tools_list_changed == Some(true) {
                log_undelivered(sink.notify_tool_list_changed().await);
            }
            if accepted.prompts_list_changed == Some(true) {
                log_undelivered(sink.notify_prompt_list_changed().await);
            }
            if accepted.resources_list_changed == Some(true) {
                log_undelivered(sink.notify_resource_list_changed().await);
            }
        }

        for peer in legacy_peers {
            log_undelivered(peer.notify_tool_list_changed().await);
            log_undelivered(peer.notify_prompt_list_changed().await);
            log_undelivered(peer.notify_resource_list_changed().await);
        }
    }
}

fn log_undelivered<E: std::fmt::Display>(result: Result<(), E>) {
    if let Err(error) = result {
        debug!(%error, "list-changed notification not delivered");
    }
}
