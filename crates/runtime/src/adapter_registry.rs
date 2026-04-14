//! Registry of live [`ChannelAdapter`] instances.
//!
//! The scheduler uses this registry to look up a running adapter by its
//! interface name when injecting platform tools for scheduler-originated turns.
//!
//! Adapters register themselves when started (via [`ChannelRunner`]) and
//! deregister on stop.  Lookup by name returns `None` if no matching adapter
//! is currently running, which the scheduler treats as graceful degradation.

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::ChannelAdapter;
use tokio::sync::RwLock;

// -- Types --

/// A thread-safe registry of live [`ChannelAdapter`] instances keyed by their
/// interface name (i.e. the value returned by [`ChannelAdapter::name()`]).
///
/// **Limitation**: only one adapter per interface name is supported.  Running
/// two adapters of the same type (e.g. two Slack workspaces) requires adapter
/// instance IDs, which is out of scope for this change.
#[derive(Clone, Default)]
pub struct AdapterRegistry {
    inner: Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a live adapter.  Overwrites any existing adapter with the same name.
    pub async fn register(&self, adapter: Arc<dyn ChannelAdapter>) {
        let name = adapter.name().to_string();
        self.inner.write().await.insert(name, adapter);
    }

    /// Deregister the adapter with the given interface name.
    pub async fn deregister(&self, name: &str) {
        self.inner.write().await.remove(name);
    }

    /// Look up a live adapter by interface name.  Returns `None` if not registered.
    pub async fn get(&self, name: &str) -> Option<Arc<dyn ChannelAdapter>> {
        self.inner.read().await.get(name).cloned()
    }
}

// -- Tests --

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::sync::Arc;

    use assistant_core::{ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, Interface};
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
}
