//! Image generation providers (`[images]`).

mod openrouter;

use std::sync::Arc;

use assistant_core::ImageGenerationProvider;
use assistant_core::types::images::{ImagesConfig, ImagesProviderKind};
use assistant_core::types::llm::{LlmConfig, OPENROUTER_API_KEY_ENV, resolve_openrouter_key};

pub use self::openrouter::{DEFAULT_MODEL, OpenRouterImages};

/// Builds the provider for `[images]`, or `None` when it is disabled. `llm`
/// supplies the OpenRouter key when `[images]` has none.
pub async fn build_image_provider(
    config: &ImagesConfig,
    llm: &LlmConfig,
) -> anyhow::Result<Option<Arc<dyn ImageGenerationProvider>>> {
    if !config.enabled {
        return Ok(None);
    }
    match config.provider {
        ImagesProviderKind::OpenRouter => {
            let api_key = resolve_openrouter_key(
                config.api_key.as_ref(),
                llm,
                OPENROUTER_API_KEY_ENV,
                |name| std::env::var(name).ok(),
            )
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "OpenRouter image generation requires an API key. Set api_key in \
                     [images] or [llm], or the OPENROUTER_API_KEY env var."
                )
            })?;
            let provider = OpenRouterImages::connect(config, api_key).await?;
            Ok(Some(Arc::new(provider)))
        }
    }
}
