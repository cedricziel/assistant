//! OpenRouter image generation (`POST {base_url}/images`).

use async_trait::async_trait;
use base64::Engine as _;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use serde_json::{Value, json};

use assistant_core::llm::call_context::LlmCallContext;
use assistant_core::types::images::{ImageOutputFormat, ImagesConfig};
use assistant_core::types::llm::OpenRouterRouting;
use assistant_core::{GeneratedImage, ImageGenerationProvider, ImageRequest, ImageResult, Secret};

use crate::openrouter::errors::http_error;
use crate::retry::{RetryConfig, classify_provider_error, parse_retry_after, with_retry_hinted};

/// OpenRouter's default image model.
pub const DEFAULT_MODEL: &str = "openai/gpt-image-2.6-flare";

pub struct OpenRouterImages {
    http: ClientWithMiddleware,
    base_url: String,
    api_key: Secret,
    model: String,
    output_format: ImageOutputFormat,
    quality: Option<String>,
    routing: Option<OpenRouterRouting>,
    pub(crate) retry: RetryConfig,
}

impl OpenRouterImages {
    pub async fn connect(config: &ImagesConfig, api_key: Secret) -> anyhow::Result<Self> {
        Ok(Self {
            http: crate::http::build_http_client(120, &RetryConfig::disabled())?,
            base_url: config
                .base_url
                .clone()
                .unwrap_or_else(|| crate::openrouter::DEFAULT_BASE_URL.to_string()),
            api_key,
            model: config
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            output_format: config.output_format,
            quality: config.quality.clone(),
            routing: config.openrouter.provider.clone(),
            retry: RetryConfig::default(),
        })
    }
}

#[async_trait]
impl ImageGenerationProvider for OpenRouterImages {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn accepts_references(&self) -> bool {
        true
    }

    async fn generate(&self, request: ImageRequest) -> anyhow::Result<ImageResult> {
        let body = self.body(&request);
        let url = format!("{}/images", self.base_url.trim_end_matches('/'));
        let response: ImagesResponse = with_retry_hinted(
            &self.retry,
            "OpenRouter",
            classify_provider_error,
            || async {
                let resp = self
                    .http
                    .post(&url)
                    .bearer_auth(self.api_key.expose())
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("OpenRouter image request failed: {e}"))?;
                let status = resp.status();
                if !status.is_success() {
                    let retry_after = resp
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(parse_retry_after);
                    let text = resp.text().await.unwrap_or_default();
                    return Err(http_error(status.as_u16(), retry_after, &text).into());
                }
                resp.json::<ImagesResponse>().await.map_err(|e| {
                    anyhow::anyhow!("Failed to parse the OpenRouter images response: {e}")
                })
            },
        )
        .await?;

        let mut images = Vec::with_capacity(response.data.len());
        for item in response.data {
            let mime = item
                .media_type
                .unwrap_or_else(|| self.output_format.mime_type().to_owned());
            let data = match (item.b64_json, item.url) {
                (Some(encoded), _) => base64::engine::general_purpose::STANDARD
                    .decode(encoded.trim())
                    .map_err(|e| {
                        anyhow::anyhow!("OpenRouter returned invalid base64 image data: {e}")
                    })?
                    .into(),
                (None, Some(url)) => self.download(&url).await?,
                (None, None) => continue,
            };
            images.push(GeneratedImage { mime, data });
        }
        Ok(ImageResult {
            images,
            cost_usd: response.usage.and_then(|u| u.cost),
        })
    }
}

impl OpenRouterImages {
    fn body(&self, request: &ImageRequest) -> Value {
        let mut body = json!({
            "model": self.model,
            "prompt": request.prompt,
            "output_format": self.output_format.as_str(),
            "response_format": "b64_json",
        });
        let quality = request.quality.as_ref().or(self.quality.as_ref());
        let fields = [
            ("aspect_ratio", request.aspect_ratio.as_ref()),
            ("resolution", request.resolution.as_ref()),
            ("quality", quality),
            ("background", request.background.as_ref()),
        ];
        for (key, value) in fields {
            if let Some(value) = value {
                body[key] = json!(value);
            }
        }
        if let Some(n) = request.n {
            body["n"] = json!(n);
        }
        if let Some(seed) = request.seed {
            body["seed"] = json!(seed);
        }
        if !request.references.is_empty() {
            body["input_references"] = Value::Array(
                request
                    .references
                    .iter()
                    .map(|r| {
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&r.data);
                        json!({"type": "image_url", "image_url": {"url": format!("data:{};base64,{encoded}", r.mime)}})
                    })
                    .collect(),
            );
        }
        if let Some(routing) = &self.routing {
            body["provider"] = json!(routing);
        }
        if let Some(context) = LlmCallContext::current() {
            body["session_id"] = json!(context.conversation_id.to_string());
            if let Some(user) = context.user_hash {
                body["user"] = json!(user);
            }
        }
        body
    }

    /// Downloads an image OpenRouter returned as a signed URL, which expires.
    async fn download(&self, url: &str) -> anyhow::Result<bytes::Bytes> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download a generated image: {e}"))?;
        if !resp.status().is_success() {
            anyhow::bail!("Downloading a generated image returned {}", resp.status());
        }
        resp.bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read a generated image: {e}"))
    }
}

#[derive(Debug, Deserialize)]
struct ImagesResponse {
    #[serde(default)]
    data: Vec<ImageItem>,
    #[serde(default)]
    usage: Option<ImagesUsage>,
}

#[derive(Debug, Deserialize)]
struct ImageItem {
    #[serde(default)]
    b64_json: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    media_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImagesUsage {
    #[serde(default)]
    cost: Option<f64>,
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use serde_json::{Value, json};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_core::llm::call_context::LlmCallContext;
    use assistant_core::types::images::{ImagesConfig, ImagesProviderKind};
    use assistant_core::types::llm::{LlmConfig, LlmProviderKind};
    use assistant_core::{ImageGenerationProvider, ImageRequest, Secret};

    use super::OpenRouterImages;
    use crate::retry::RetryConfig;

    const PNG: &[u8] = b"\\x89PNG-bytes";

    fn config(server: &MockServer) -> ImagesConfig {
        ImagesConfig {
            provider: ImagesProviderKind::OpenRouter,
            enabled: true,
            model: None,
            base_url: Some(server.uri()),
            api_key: None,
            output_format: Default::default(),
            quality: None,
            openrouter: Default::default(),
        }
    }

    async fn provider(server: &MockServer) -> OpenRouterImages {
        let mut provider = OpenRouterImages::connect(&config(server), Secret::new("sk-or"))
            .await
            .unwrap();
        provider.retry = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 10,
            jitter: false,
            ..RetryConfig::default()
        };
        provider
    }

    fn b64(data: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(data)
    }

    async fn mount_images(server: &MockServer, body: Value) {
        Mock::given(method("POST"))
            .and(path("/images"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(server)
            .await;
    }

    async fn sent_body(server: &MockServer) -> Value {
        let requests = server.received_requests().await.unwrap();
        let request = requests.iter().find(|r| r.url.path() == "/images").unwrap();
        serde_json::from_slice(&request.body).unwrap()
    }

    fn lighthouse() -> ImageRequest {
        ImageRequest {
            prompt: "a lighthouse at dusk".into(),
            aspect_ratio: Some("16:9".into()),
            ..ImageRequest::default()
        }
    }

    #[tokio::test]
    async fn posts_the_request_and_decodes_base64_images() {
        let server = MockServer::start().await;
        mount_images(
            &server,
            json!({"created": 1, "data": [{"b64_json": b64(PNG), "media_type": "image/png"}],
                   "usage": {"prompt_tokens": 0, "completion_tokens": 10, "total_tokens": 10, "cost": 0.04}}),
        )
        .await;

        let result = provider(&server)
            .await
            .generate(lighthouse())
            .await
            .unwrap();

        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].mime, "image/png");
        assert_eq!(&result.images[0].data[..], PNG);
        assert_eq!(result.cost_usd, Some(0.04));
        assert_eq!(
            sent_body(&server).await,
            json!({
                "model": "openai/gpt-image-2.6-flare",
                "prompt": "a lighthouse at dusk",
                "aspect_ratio": "16:9",
                "output_format": "png",
                "response_format": "b64_json",
            })
        );
    }

    #[tokio::test]
    async fn url_entries_are_downloaded() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/generated/1.webp"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(b"RIFF-webp".to_vec(), "image/webp"),
            )
            .mount(&server)
            .await;
        mount_images(
            &server,
            json!({"created": 1, "data": [{"url": format!("{}/generated/1.webp", server.uri()), "media_type": "image/webp"}]}),
        )
        .await;

        let result = provider(&server)
            .await
            .generate(lighthouse())
            .await
            .unwrap();

        assert_eq!(result.images[0].mime, "image/webp");
        assert_eq!(&result.images[0].data[..], b"RIFF-webp");
    }

    #[tokio::test]
    async fn references_are_sent_as_data_uris_with_turn_identity() {
        let server = MockServer::start().await;
        mount_images(
            &server,
            json!({"created": 1, "data": [{"b64_json": b64(PNG)}]}),
        )
        .await;
        let request = ImageRequest {
            prompt: "make the sky purple".into(),
            n: Some(2),
            references: vec![assistant_core::ReferenceImage {
                mime: "image/jpeg".into(),
                data: bytes::Bytes::from_static(b"jpeg"),
            }],
            ..ImageRequest::default()
        };
        let context = LlmCallContext {
            conversation_id: Default::default(),
            user_hash: Some("u-hash".into()),
        };

        let result = context
            .scope(async { provider(&server).await.generate(request).await })
            .await
            .unwrap();

        assert_eq!(
            result.images[0].mime, "image/png",
            "the configured format when omitted"
        );
        let body = sent_body(&server).await;
        assert_eq!(body["n"], 2);
        assert_eq!(
            body["input_references"],
            json!([{"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{}", b64(b"jpeg"))}}])
        );
        assert_eq!(body["session_id"], "00000000-0000-0000-0000-000000000000");
        assert_eq!(body["user"], "u-hash");
    }

    #[tokio::test]
    async fn out_of_credits_says_so_and_rate_limits_are_retried() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/images"))
            .respond_with(
                ResponseTemplate::new(402).set_body_json(
                    json!({"error": {"code": 402, "message": "Insufficient credits"}}),
                ),
            )
            .mount(&server)
            .await;
        let err = provider(&server)
            .await
            .generate(lighthouse())
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("lacks credits"), "{err}");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/images"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "1")
                    .set_body_json(json!({"error": {"code": 429, "message": "slow down"}})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount_images(
            &server,
            json!({"created": 1, "data": [{"b64_json": b64(PNG)}]}),
        )
        .await;
        let started = std::time::Instant::now();
        provider(&server)
            .await
            .generate(lighthouse())
            .await
            .unwrap();
        assert!(started.elapsed() >= std::time::Duration::from_secs(1));
    }

    #[tokio::test]
    async fn builder_reuses_the_llm_key_and_skips_disabled_configs() {
        let server = MockServer::start().await;
        let llm = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some(Secret::new("sk-or-chat")),
            ..LlmConfig::default()
        };
        let built = crate::images::build_image_provider(&config(&server), &llm)
            .await
            .unwrap();
        assert_eq!(
            built.map(|p| p.name().to_string()).as_deref(),
            Some("openrouter")
        );

        let disabled = ImagesConfig {
            enabled: false,
            ..config(&server)
        };
        assert!(
            crate::images::build_image_provider(&disabled, &llm)
                .await
                .unwrap()
                .is_none()
        );
    }
}
