//! What the configured OpenRouter model can do, from the model catalog.
//!
//! The catalog is fetched in the background when the provider is built.
//! `capabilities()` is synchronous, so the result lives in an [`ArcSwapOption`]
//! that it can read without blocking. The first chat call waits for the fetch
//! for at most [`READY_TIMEOUT`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use tokio::sync::watch;
use tracing::{debug, error, warn};

use assistant_core::types::llm::OpenRouterRouting;
use assistant_core::{Clock, Secret};

use crate::retry::RetryConfig;

/// How long the first chat call waits for the catalog.
const READY_TIMEOUT: Duration = Duration::from_secs(5);

/// The shortest wait before a failed fetch is retried.
const RETRY_INTERVAL: Duration = Duration::from_secs(10 * 60);

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
    clock: Arc<dyn Clock>,
    info: ArcSwapOption<ModelInfo>,
    /// Set when the catalog does not list the model; calls then fail.
    unknown_model: AtomicBool,
    /// When a failed fetch may be retried.
    retry_at: Mutex<Option<Instant>>,
    in_flight: AtomicBool,
    /// `false` while a fetch runs.
    ready: watch::Sender<bool>,
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
        clock: Arc<dyn Clock>,
    ) -> anyhow::Result<Arc<Self>> {
        let catalog = Arc::new(Self {
            http: crate::http::build_http_client(timeout_secs, &RetryConfig::disabled())?,
            base_url,
            api_key,
            model,
            routing,
            clock,
            info: ArcSwapOption::empty(),
            unknown_model: AtomicBool::new(false),
            retry_at: Mutex::new(None),
            in_flight: AtomicBool::new(false),
            ready: watch::Sender::new(false),
        });
        catalog.refresh_if_due();
        Ok(catalog)
    }

    /// The model's capabilities, or `None` until a fetch has succeeded.
    pub fn info(&self) -> Option<Arc<ModelInfo>> {
        self.info.load_full()
    }

    /// Fails when the catalog does not list the configured model.
    pub fn check_model(&self) -> anyhow::Result<()> {
        if self.unknown_model.load(Ordering::SeqCst) {
            anyhow::bail!("OpenRouter has no model `{}`", self.model);
        }
        Ok(())
    }

    /// Starts a fetch when one is due, then waits for any running fetch for
    /// at most [`READY_TIMEOUT`].
    pub async fn wait_ready(self: &Arc<Self>) {
        self.refresh_if_due();
        let mut ready = self.ready.subscribe();
        if tokio::time::timeout(READY_TIMEOUT, ready.wait_for(|done| *done))
            .await
            .is_err()
        {
            warn!(model = %self.model, "OpenRouter model catalog is slow; continuing without it");
        }
    }

    /// A fetch is due until one succeeds or finds the model missing, and
    /// after a failure only once [`RETRY_INTERVAL`] has passed.
    fn refresh_if_due(self: &Arc<Self>) {
        if tokio::runtime::Handle::try_current().is_err()
            || self.info.load().is_some()
            || self.unknown_model.load(Ordering::SeqCst)
        {
            return;
        }
        let now = self.clock.now_instant();
        let due = self
            .retry_at
            .lock()
            .map(|at| at.is_none_or(|at| now >= at))
            .unwrap_or(true);
        if !due || self.in_flight.swap(true, Ordering::SeqCst) {
            return;
        }
        self.ready.send_replace(false);
        let catalog = Arc::clone(self);
        tokio::spawn(async move {
            catalog.refresh().await;
            catalog.in_flight.store(false, Ordering::SeqCst);
            catalog.ready.send_replace(true);
        });
    }

    async fn refresh(&self) {
        match self.fetch().await {
            Ok(Some(info)) => {
                debug!(model = %self.model, ?info, "OpenRouter model catalog loaded");
                self.info.store(Some(Arc::new(info)));
            }
            Ok(None) => {
                error!(model = %self.model, "OpenRouter has no such model; chat calls will fail");
                self.unknown_model.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                warn!(
                    model = %self.model,
                    error = %e,
                    "OpenRouter model catalog unavailable; using conservative capabilities"
                );
                if let Ok(mut at) = self.retry_at.lock() {
                    *at = Some(self.clock.now_instant() + RETRY_INTERVAL);
                }
            }
        }
    }

    /// The model's capabilities, or `None` when the catalog does not list it.
    async fn fetch(&self) -> anyhow::Result<Option<ModelInfo>> {
        let base = self.base_url.trim_end_matches('/');
        let models: ModelList = self.get_json(&format!("{base}/models")).await?;
        let Some(entry) = models.data.into_iter().find(|m| m.id == self.model) else {
            return Ok(None);
        };
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
        Ok(Some(model_info(&entry, &endpoints, self.routing.as_ref())))
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
