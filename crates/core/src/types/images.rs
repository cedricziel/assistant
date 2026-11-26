//! Image generation configuration (`[images]` in `org.toml`).

use serde::{Deserialize, Serialize};

use super::default_true;
use super::llm::OpenRouterRouting;
use crate::secret::Secret;

/// Which image generation backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImagesProviderKind {
    /// OpenRouter's image generation endpoint (`POST /images`).
    OpenRouter,
}

/// The file format generated images are returned in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageOutputFormat {
    #[default]
    Png,
    Jpeg,
    Webp,
}

impl ImageOutputFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Webp => "webp",
        }
    }

    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }
}

/// `[images]`: enables the `image-generate` tool.
///
/// ```toml
/// [images]
/// provider = "openrouter"
/// # model = "openai/gpt-image-2.6-flare"
/// # api_key = "sk-or-..."  # or reuse the [llm] OpenRouter key
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagesConfig {
    pub provider: ImagesProviderKind,
    /// When `false`, no provider is built and the tool isn't offered.
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<Secret>,
    #[serde(default)]
    pub output_format: ImageOutputFormat,
    /// Default `quality` for requests that don't set one.
    #[serde(default)]
    pub quality: Option<String>,
    /// OpenRouter-only settings (`[images.openrouter]`).
    #[serde(default)]
    pub openrouter: ImagesOpenRouterOptions,
}

/// `[images.openrouter]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagesOpenRouterOptions {
    /// Provider routing, with the same keys as `[llm.openrouter.provider]`.
    pub provider: Option<OpenRouterRouting>,
}

#[cfg(test)]
mod tests {
    use super::{ImageOutputFormat, ImagesConfig, ImagesProviderKind};
    use crate::types::llm::OpenRouterDataCollection;

    #[test]
    fn minimal_config_takes_the_defaults() {
        let cfg: ImagesConfig = toml::from_str("provider = \"openrouter\"").unwrap();
        assert_eq!(cfg.provider, ImagesProviderKind::OpenRouter);
        assert!(cfg.enabled);
        assert_eq!(cfg.output_format, ImageOutputFormat::Png);
        assert_eq!(cfg.model, None);
    }

    #[test]
    fn full_config_with_routing_parses() {
        let cfg: ImagesConfig = toml::from_str(
            r#"
            provider = "openrouter"
            enabled = false
            model = "google/gemini-3.5-flash-image"
            api_key = "sk-or-img"
            output_format = "webp"
            quality = "high"
            [openrouter.provider]
            data_collection = "deny"
            "#,
        )
        .unwrap();
        assert!(!cfg.enabled);
        assert_eq!(cfg.output_format, ImageOutputFormat::Webp);
        assert_eq!(cfg.quality.as_deref(), Some("high"));
        let routing = cfg.openrouter.provider.unwrap();
        assert_eq!(
            routing.data_collection,
            Some(OpenRouterDataCollection::Deny)
        );
    }

    #[test]
    fn unknown_keys_and_formats_are_rejected() {
        assert!(
            toml::from_str::<ImagesConfig>("provider = \"openrouter\"\nsize = \"2K\"").is_err()
        );
        assert!(
            toml::from_str::<ImagesConfig>("provider = \"openrouter\"\noutput_format = \"tiff\"")
                .is_err()
        );
        assert!(toml::from_str::<ImagesConfig>("provider = \"dalle\"").is_err());
    }
}
