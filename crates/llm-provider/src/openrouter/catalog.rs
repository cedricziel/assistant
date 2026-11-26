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

use assistant_core::types::llm::{OpenRouterEffort, OpenRouterRouting};
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
    /// The model reads PDFs itself (`file` input).
    pub file_input: bool,
    pub tools: bool,
    pub tool_choice: bool,
    pub parallel_tool_calls: bool,
    /// `structured_outputs` or `response_format` is supported.
    pub json_schema: bool,
}

/// What the configuration needs from the model; a model without it fails
/// every call.
#[derive(Debug, Clone, Copy, Default)]
pub struct Requirements {
    pub effort: Option<OpenRouterEffort>,
    pub image_output: bool,
}

pub struct OpenRouterCatalog {
    http: ClientWithMiddleware,
    base_url: String,
    api_key: Secret,
    model: String,
    routing: Option<OpenRouterRouting>,
    requirements: Requirements,
    clock: Arc<dyn Clock>,
    info: ArcSwapOption<ModelInfo>,
    /// Why the configuration can't work with this model, such as an unknown
    /// model, an unsupported effort or missing image output; calls then fail
    /// with it.
    config_error: ArcSwapOption<String>,
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
        requirements: Requirements,
        clock: Arc<dyn Clock>,
    ) -> anyhow::Result<Arc<Self>> {
        let catalog = Arc::new(Self {
            http: crate::http::build_http_client(timeout_secs, &RetryConfig::disabled())?,
            base_url,
            api_key,
            model,
            routing,
            requirements,
            clock,
            info: ArcSwapOption::empty(),
            config_error: ArcSwapOption::empty(),
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

    /// Fails when the catalog showed the configuration can't work: the
    /// model is unknown, or it lacks a required capability.
    pub fn check_model(&self) -> anyhow::Result<()> {
        match self.config_error.load_full() {
            Some(error) => Err(anyhow::anyhow!("{error}")),
            None => Ok(()),
        }
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
            || self.config_error.load().is_some()
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
            Ok(Ok(info)) => {
                debug!(model = %self.model, ?info, "OpenRouter model catalog loaded");
                self.info.store(Some(Arc::new(info)));
            }
            Ok(Err(problem)) => {
                error!(model = %self.model, "{problem}; chat calls will fail");
                self.config_error.store(Some(Arc::new(problem)));
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

    /// The model's capabilities, or why the configuration can't work with it.
    async fn fetch(&self) -> anyhow::Result<Result<ModelInfo, String>> {
        let base = self.base_url.trim_end_matches('/');
        let models: ModelList = self.get_json(&format!("{base}/models")).await?;
        let Some(entry) = models.data.into_iter().find(|m| m.id == self.model) else {
            return Ok(Err(format!("OpenRouter has no model `{}`", self.model)));
        };
        if let Some(problem) = effort_problem(
            &self.model,
            self.requirements.effort,
            entry.reasoning.as_ref(),
        ) {
            return Ok(Err(problem));
        }
        if self.requirements.image_output && !entry.outputs_images() {
            return Ok(Err(format!(
                "OpenRouter model `{}` does not output images; \
                 remove `image_output` from [llm.openrouter]",
                self.model
            )));
        }
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
        Ok(Ok(model_info(&entry, &endpoints, self.routing.as_ref())))
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
    /// Absent when the model has no effort selection.
    #[serde(default)]
    reasoning: Option<CatalogReasoning>,
}

#[derive(Debug, Default, Deserialize)]
struct CatalogReasoning {
    /// `None` means every effort is accepted.
    #[serde(default)]
    supported_efforts: Option<Vec<String>>,
    /// When set, `effort = "none"` is rejected.
    #[serde(default)]
    mandatory: bool,
}

/// Why `effort` can't be used with `model`, if it can't.
fn effort_problem(
    model: &str,
    effort: Option<OpenRouterEffort>,
    reasoning: Option<&CatalogReasoning>,
) -> Option<String> {
    let effort = effort?;
    let Some(reasoning) = reasoning else {
        return Some(format!(
            "OpenRouter model `{model}` does not support reasoning effort; \
             remove `effort` from [llm.openrouter.reasoning]"
        ));
    };
    if reasoning.mandatory && effort == OpenRouterEffort::None {
        return Some(format!(
            "reasoning is mandatory for OpenRouter model `{model}`, so effort `none` is not allowed"
        ));
    }
    match &reasoning.supported_efforts {
        Some(supported) if !supported.iter().any(|e| e == effort.as_str()) => Some(format!(
            "OpenRouter model `{model}` does not support reasoning effort `{}`; it supports {}",
            effort.as_str(),
            supported.join(", ")
        )),
        _ => None,
    }
}

#[derive(Debug, Default, Deserialize)]
struct Architecture {
    #[serde(default)]
    input_modalities: Option<Vec<String>>,
    #[serde(default)]
    output_modalities: Option<Vec<String>>,
}

impl ModelEntry {
    fn outputs_images(&self) -> bool {
        self.architecture
            .as_ref()
            .and_then(|a| a.output_modalities.as_deref())
            .is_some_and(|m| m.iter().any(|m| m == "image"))
    }
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
    let has = |name: &str| params.iter().any(|p| p == name);
    ModelInfo {
        context_window_tokens,
        vision: inputs.iter().any(|m| m == "image"),
        file_input: inputs.iter().any(|m| m == "file"),
        tools: has("tools"),
        tool_choice: has("tool_choice"),
        parallel_tool_calls: has("parallel_tool_calls"),
        json_schema: has("structured_outputs") || has("response_format"),
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
