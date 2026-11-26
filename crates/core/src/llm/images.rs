//! The image generation provider trait and its request and result types.

use async_trait::async_trait;

/// An image the request edits or takes as a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceImage {
    pub mime: String,
    pub data: bytes::Bytes,
}

/// A request for one or more generated images. Unset fields leave the
/// choice to the provider.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageRequest {
    pub prompt: String,
    /// How many images to generate.
    pub n: Option<u32>,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
    pub quality: Option<String>,
    pub background: Option<String>,
    pub seed: Option<i64>,
    pub references: Vec<ReferenceImage>,
}

/// One generated image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedImage {
    pub mime: String,
    pub data: bytes::Bytes,
}

/// What a generation produced. Providers may return fewer images than asked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageResult {
    pub images: Vec<GeneratedImage>,
    /// Cost of the request in US dollars, when the provider reports it.
    pub cost_usd: Option<f64>,
}

/// Generates images from a prompt and optional reference images.
#[async_trait]
pub trait ImageGenerationProvider: Send + Sync {
    /// Short provider name, e.g. `"openrouter"`.
    fn name(&self) -> &str;

    /// Whether requests may carry reference images.
    fn accepts_references(&self) -> bool;

    async fn generate(&self, request: ImageRequest) -> anyhow::Result<ImageResult>;
}
