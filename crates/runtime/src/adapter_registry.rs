//! Registry of live [`ChannelAdapter`] instances.
//!
//! The scheduler uses this registry to look up a running adapter by its
//! interface name or instance ID when injecting platform tools for
//! scheduler-originated turns.
//!
//! Adapters register themselves when started (via [`ChannelRunner`]) and
//! deregister on stop.  Lookup returns `None` if no matching adapter is
//! currently running, which the scheduler treats as graceful degradation.
//!
//! ## Registration modes
//!
//! - **By name** ([`register`](AdapterRegistry::register)) — legacy mode,
//!   keys by `adapter.name()`. Only one adapter per interface type.
//! - **By instance ID** ([`register_instance`](AdapterRegistry::register_instance))
//!   — multi-instance mode, allows multiple adapters of the same type (e.g.
//!   two Slack workspaces).

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::ChannelAdapter;
use tokio::sync::RwLock;

// -- Types --

/// Stored adapter entry with its interface name for reverse lookups.
struct AdapterEntry {
    adapter: Arc<dyn ChannelAdapter>,
    interface_name: String,
}

/// A thread-safe registry of live [`ChannelAdapter`] instances.
///
/// Supports two keying modes:
/// - **By name** (legacy): one adapter per interface type, keyed by
///   [`ChannelAdapter::name()`].
/// - **By instance ID** (multi-instance): multiple adapters of the same
///   type, keyed by a caller-supplied instance ID string.
#[derive(Clone, Default)]
pub struct AdapterRegistry {
    /// Legacy name-based registrations.
    by_name: Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>>,
    /// Instance-based registrations (multi-instance support).
    by_instance: Arc<RwLock<HashMap<String, AdapterEntry>>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // -- Legacy name-based API ------------------------------------------------

    /// Register a live adapter keyed by its interface name.
    ///
    /// Overwrites any existing adapter with the same name.
    pub async fn register(&self, adapter: Arc<dyn ChannelAdapter>) {
        let name = adapter.name().to_string();
        self.by_name.write().await.insert(name, adapter);
    }

    /// Deregister the adapter with the given interface name.
    pub async fn deregister(&self, name: &str) {
        self.by_name.write().await.remove(name);
    }

    /// Look up a live adapter by interface name.
    ///
    /// Checks the legacy name-based registry first, then falls back to
    /// searching instance-based registrations by interface name.
    pub async fn get(&self, name: &str) -> Option<Arc<dyn ChannelAdapter>> {
        // Try legacy registry first.
        if let Some(a) = self.by_name.read().await.get(name).cloned() {
            return Some(a);
        }
        // Fall back to instance registry — return the first matching name.
        self.by_instance
            .read()
            .await
            .values()
            .find(|e| e.interface_name == name)
            .map(|e| e.adapter.clone())
    }

    // -- Instance-based API ---------------------------------------------------

    /// Register an adapter with an explicit instance ID.
    ///
    /// This allows multiple adapters of the same interface type (e.g. two
    /// Slack workspaces) to coexist in the registry.
    pub async fn register_instance(&self, instance_id: &str, adapter: Arc<dyn ChannelAdapter>) {
        let entry = AdapterEntry {
            interface_name: adapter.name().to_string(),
            adapter,
        };
        self.by_instance
            .write()
            .await
            .insert(instance_id.to_string(), entry);
    }

    /// Deregister the adapter with the given instance ID.
    pub async fn deregister_instance(&self, instance_id: &str) {
        self.by_instance.write().await.remove(instance_id);
    }

    /// Look up a live adapter by instance ID.
    pub async fn get_by_instance(&self, instance_id: &str) -> Option<Arc<dyn ChannelAdapter>> {
        self.by_instance
            .read()
            .await
            .get(instance_id)
            .map(|e| e.adapter.clone())
    }
}

// -- Tests --

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::sync::Arc;

    use assistant_core::{
        ChannelAdapter, ChannelContent, ChannelMessage, types::conversation::ChannelType,
        types::conversation::Interface,
    };
    use async_trait::async_trait;
    use futures::Stream;

    use super::AdapterRegistry;

    struct FakeAdapter {
        name: &'static str,
        channel_type: ChannelType,
    }

    #[async_trait]
    impl ChannelAdapter for FakeAdapter {
        fn name(&self) -> &str {
            self.name
        }
        fn channel_type(&self) -> ChannelType {
            self.channel_type.clone()
        }
        fn interface(&self) -> Interface {
            Interface::Slack
        }
        async fn start(
            &self,
        ) -> anyhow::Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>> {
            unimplemented!()
        }
        async fn send(
            &self,
            _user: &assistant_core::ChannelUser,
            _content: ChannelContent,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn stop(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn register_and_get() {
        let registry = AdapterRegistry::new();
        let adapter: Arc<dyn ChannelAdapter> = Arc::new(FakeAdapter {
            name: "slack",
            channel_type: ChannelType::Slack,
        });

        registry.register(adapter).await;
        assert!(
            registry.get("slack").await.is_some(),
            "registered adapter should be retrievable"
        );
        assert!(
            registry.get("signal").await.is_none(),
            "unregistered adapter should return None"
        );
    }

    #[tokio::test]
    async fn deregister_removes_adapter() {
        let registry = AdapterRegistry::new();
        let adapter: Arc<dyn ChannelAdapter> = Arc::new(FakeAdapter {
            name: "slack",
            channel_type: ChannelType::Slack,
        });

        registry.register(adapter).await;
        registry.deregister("slack").await;
        assert!(
            registry.get("slack").await.is_none(),
            "deregistered adapter should not be retrievable"
        );
    }

    #[tokio::test]
    async fn get_unknown_returns_none() {
        let registry = AdapterRegistry::new();
        assert!(registry.get("matrix").await.is_none());
    }

    #[tokio::test]
    async fn register_with_instance_id_and_lookup() {
        let registry = AdapterRegistry::new();
        let adapter: Arc<dyn ChannelAdapter> = Arc::new(FakeAdapter {
            name: "slack",
            channel_type: ChannelType::Slack,
        });

        registry
            .register_instance("inst_workspace_a", adapter)
            .await;

        assert!(
            registry.get_by_instance("inst_workspace_a").await.is_some(),
            "adapter should be retrievable by instance ID"
        );
        assert!(
            registry.get_by_instance("inst_workspace_b").await.is_none(),
            "unknown instance ID should return None"
        );
        // Legacy name-based lookup should also find it.
        assert!(
            registry.get("slack").await.is_some(),
            "name-based lookup should still work"
        );
    }

    #[tokio::test]
    async fn multiple_instances_same_interface_type() {
        let registry = AdapterRegistry::new();
        let a1: Arc<dyn ChannelAdapter> = Arc::new(FakeAdapter {
            name: "slack",
            channel_type: ChannelType::Slack,
        });
        let a2: Arc<dyn ChannelAdapter> = Arc::new(FakeAdapter {
            name: "slack",
            channel_type: ChannelType::Slack,
        });

        registry.register_instance("ws_a", a1).await;
        registry.register_instance("ws_b", a2).await;

        assert!(
            registry.get_by_instance("ws_a").await.is_some(),
            "first instance should be retrievable"
        );
        assert!(
            registry.get_by_instance("ws_b").await.is_some(),
            "second instance should be retrievable"
        );
    }

    #[tokio::test]
    async fn deregister_instance() {
        let registry = AdapterRegistry::new();
        let adapter: Arc<dyn ChannelAdapter> = Arc::new(FakeAdapter {
            name: "slack",
            channel_type: ChannelType::Slack,
        });

        registry.register_instance("inst_1", adapter).await;
        registry.deregister_instance("inst_1").await;

        assert!(
            registry.get_by_instance("inst_1").await.is_none(),
            "deregistered instance should not be retrievable"
        );
    }
}
