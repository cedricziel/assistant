//! What the configured OpenRouter model can do, from the model catalog.
//!
//! The catalog is fetched in the background when the provider is built.
//! `capabilities()` is synchronous, so the result lives in an [`ArcSwapOption`]
//! that it can read without blocking. The first chat call waits for the fetch
//! for at most [`READY_TIMEOUT`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use arc_swap::ArcSwapOption;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use tokio::sync::watch;
use tracing::{debug, warn};

use assistant_core::Secret;
use assistant_core::types::llm::OpenRouterRouting;

use crate::retry::RetryConfig;

/// How long the first chat call waits for the catalog.
const READY_TIMEOUT: Duration = Duration::from_secs(5);

/// The configured model's capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    /// The smallest context length among the endpoints routing can select.
    pub context_window_tokens: Option<u64>,
    pub vision: bool,
    pub tools: bool,
}

pub struct OpenRouterCatalog {
    http: ClientWithMiddleware,
    base_url: String,
    api_key: Secret,
    model: String,
    routing: Option<OpenRouterRouting>,
    info: ArcSwapOption<ModelInfo>,
    ready: watch::Sender<bool>,
    started: AtomicBool,
}

impl OpenRouterCatalog {
    /// Builds the catalog and starts the fetch when a Tokio runtime is
    /// running; otherwise the fetch starts on the first [`wait_ready`].
    ///
    /// [`wait_ready`]: Self::wait_ready
    pub fn new(
        base_url: String,
        api_key: Secret,
        timeout_secs: u64,
        model: String,
        routing: Option<OpenRouterRouting>,
    ) -> anyhow::Result<Arc<Self>> {
        let catalog = Arc::new(Self {
            http: crate::http::build_http_client(timeout_secs, &RetryConfig::disabled())?,
            base_url,
            api_key,
            model,
            routing,
            info: ArcSwapOption::empty(),
            ready: watch::Sender::new(false),
            started: AtomicBool::new(false),
        });
        catalog.start();
        Ok(catalog)
    }

    /// The model's capabilities, or `None` until a fetch has succeeded.
    pub fn info(&self) -> Option<Arc<ModelInfo>> {
        self.info.load_full()
    }

    /// Waits until the first fetch has finished, for at most [`READY_TIMEOUT`].
    pub async fn wait_ready(self: &Arc<Self>) {
        self.start();
        let mut ready = self.ready.subscribe();
        if tokio::time::timeout(READY_TIMEOUT, ready.wait_for(|done| *done))
            .await
            .is_err()
        {
            warn!(model = %self.model, "OpenRouter model catalog is slow; continuing without it");
        }
    }

    fn start(self: &Arc<Self>) {
        if tokio::runtime::Handle::try_current().is_err()
            || self.started.swap(true, Ordering::SeqCst)
        {
            return;
        }
        let catalog = Arc::clone(self);
        tokio::spawn(async move { catalog.refresh().await });
    }

    async fn refresh(&self) {
        match self.fetch().await {
            Ok(info) => {
                debug!(model = %self.model, ?info, "OpenRouter model catalog loaded");
                self.info.store(Some(Arc::new(info)));
            }
            Err(e) => warn!(
                model = %self.model,
                error = %e,
                "OpenRouter model catalog unavailable; using conservative capabilities"
            ),
        }
        self.ready.send_replace(true);
    }

    async fn fetch(&self) -> anyhow::Result<ModelInfo> {
        let base = self.base_url.trim_end_matches('/');
        let models: ModelList = self.get_json(&format!("{base}/models")).await?;
        let entry = models
            .data
            .into_iter()
            .find(|m| m.id == self.model)
            .ok_or_else(|| anyhow::anyhow!("OpenRouter has no model `{}`", self.model))?;
        let endpoints = match self
            .get_json::<EndpointList>(&format!("{base}/models/{}/endpoints", self.model))
            .await
        {
            Ok(list) => list.data.endpoints,
            Err(e) => {
                warn!(model = %self.model, error = %e, "OpenRouter endpoints unavailable");
                Vec::new()
            }
        };
        Ok(model_info(&entry, &endpoints, self.routing.as_ref()))
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> anyhow::Result<T> {
        let resp = self
            .http
            .get(url)
            .bearer_auth(self.api_key.expose())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenRouter catalog request failed: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("OpenRouter catalog returned {status}");
        }
        resp.json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse the OpenRouter catalog: {e}"))
    }
}

// -- Catalog wire types --

#[derive(Debug, Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    context_length: Option<u64>,
    #[serde(default)]
    architecture: Option<Architecture>,
    #[serde(default)]
    supported_parameters: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct Architecture {
    #[serde(default)]
    input_modalities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct EndpointList {
    data: EndpointData,
}

#[derive(Debug, Deserialize)]
struct EndpointData {
    #[serde(default)]
    endpoints: Vec<Endpoint>,
}

#[derive(Debug, Deserialize)]
struct Endpoint {
    #[serde(default)]
    provider_name: String,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    context_length: Option<u64>,
}

impl Endpoint {
    /// Whether routing slug `slug` (e.g. `anthropic`) names this endpoint.
    fn matches(&self, slug: &str) -> bool {
        let slug = slug.to_lowercase();
        let tag = self.tag.to_lowercase();
        tag == slug
            || tag.starts_with(&format!("{slug}/"))
            || self.provider_name.to_lowercase() == slug
    }
}

fn model_info(
    entry: &ModelEntry,
    endpoints: &[Endpoint],
    routing: Option<&OpenRouterRouting>,
) -> ModelInfo {
    let selectable = selectable_endpoints(endpoints, routing);
    let context_window_tokens = selectable
        .iter()
        .filter_map(|e| e.context_length)
        .min()
        .or(entry.context_length);
    let inputs = entry
        .architecture
        .as_ref()
        .and_then(|a| a.input_modalities.as_deref())
        .unwrap_or_default();
    let params = entry.supported_parameters.as_deref().unwrap_or_default();
    ModelInfo {
        context_window_tokens,
        vision: inputs.iter().any(|m| m == "image"),
        tools: params.iter().any(|p| p == "tools"),
    }
}

/// The endpoints routing can select: `ignore` removes endpoints, `only`
/// keeps the named ones, and `order` keeps the named ones when fallbacks
/// are off.
fn selectable_endpoints<'a>(
    endpoints: &'a [Endpoint],
    routing: Option<&OpenRouterRouting>,
) -> Vec<&'a Endpoint> {
    let Some(routing) = routing else {
        return endpoints.iter().collect();
    };
    let named = |list: &Option<Vec<String>>, e: &Endpoint| {
        list.as_ref()
            .is_some_and(|slugs| slugs.iter().any(|s| e.matches(s)))
    };
    let pinned = if routing.only.is_some() {
        routing.only.as_ref()
    } else if routing.allow_fallbacks == Some(false) {
        routing.order.as_ref()
    } else {
        None
    };
    endpoints
        .iter()
        .filter(|e| !named(&routing.ignore, e))
        .filter(|e| pinned.is_none_or(|slugs| slugs.iter().any(|s| e.matches(s))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Endpoint;

    fn endpoint(provider_name: &str, tag: &str) -> Endpoint {
        Endpoint {
            provider_name: provider_name.into(),
            tag: tag.into(),
            context_length: None,
        }
    }

    #[test]
    fn slug_matches_tag_variant_and_provider_name() {
        assert!(endpoint("DeepInfra", "deepinfra/fp8").matches("deepinfra"));
        assert!(endpoint("Anthropic", "anthropic").matches("Anthropic"));
        assert!(endpoint("Google", "google-vertex").matches("google"));
        assert!(!endpoint("Google", "google-vertex").matches("anthropic"));
    }
}
