//! `image-generate` tool: create or edit images with the `[images]` provider.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;
use uuid::Uuid;

use assistant_core::attachment::extension_for_mime;
use assistant_core::types::conversation::ExecutionContext;
use assistant_core::{
    Attachment, ImageGenerationProvider, ImageRequest, ReferenceImage, ToolHandler, ToolOutput,
};
use assistant_storage::AttachmentStore;

/// Most reference images one request may carry.
const MAX_REFERENCES: usize = 16;

pub struct ImageGenerateHandler {
    provider: Arc<dyn ImageGenerationProvider>,
    attachments: Arc<dyn AttachmentStore>,
}

impl ImageGenerateHandler {
    pub fn new(
        provider: Arc<dyn ImageGenerationProvider>,
        attachments: Arc<dyn AttachmentStore>,
    ) -> Self {
        Self {
            provider,
            attachments,
        }
    }
}

#[async_trait]
impl ToolHandler for ImageGenerateHandler {
    fn name(&self) -> &str {
        "image-generate"
    }

    fn description(&self) -> &str {
        "Generate images from a text prompt, or edit images the user attached by passing \
         their attachment IDs. The images are sent to the user with your reply."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {"type": "string", "description": "What to draw, or how to change the reference images."},
                "n": {"type": "integer", "minimum": 1, "maximum": 4, "description": "How many images to generate (default 1)."},
                "aspect_ratio": {"type": "string", "description": "For example \"1:1\", \"16:9\" or \"9:16\"."},
                "resolution": {"type": "string", "description": "For example \"1K\", \"2K\" or \"4K\"."},
                "quality": {"type": "string", "description": "For example \"low\", \"medium\" or \"high\"."},
                "background": {"type": "string", "description": "\"auto\", \"transparent\" or \"opaque\"."},
                "seed": {"type": "integer", "description": "Seed for repeatable results, where the model supports it."},
                "reference_attachment_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": MAX_REFERENCES,
                    "description": "IDs of images attached in this conversation to edit or use as references."
                }
            },
            "required": ["prompt"]
        })
    }

    async fn run(
        &self,
        params: HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let Some(prompt) = params
            .get("prompt")
            .and_then(Value::as_str)
            .filter(|p| !p.trim().is_empty())
        else {
            return Ok(ToolOutput::error("image-generate: `prompt` is required"));
        };
        let text = |key: &str| params.get(key).and_then(Value::as_str).map(str::to_owned);
        let n = params
            .get("n")
            .and_then(Value::as_u64)
            .map(|n| n.clamp(1, 4) as u32);
        let references = match self.references(&params, ctx).await {
            Ok(references) => references,
            Err(message) => return Ok(ToolOutput::error(message)),
        };
        if !references.is_empty() && !self.provider.accepts_references() {
            return Ok(ToolOutput::error(
                "image-generate: the configured image model does not take reference images",
            ));
        }
        let request = ImageRequest {
            prompt: prompt.to_owned(),
            n,
            aspect_ratio: text("aspect_ratio"),
            resolution: text("resolution"),
            quality: text("quality"),
            background: text("background"),
            seed: params.get("seed").and_then(Value::as_i64),
            references,
        };

        let result = match self.provider.generate(request).await {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, "image-generate failed");
                return Ok(ToolOutput::error(format!("Image generation failed: {e}")));
            }
        };
        let requested = n.unwrap_or(1) as usize;
        let produced = result.images.len();
        let content = if produced < requested {
            format!(
                "Generated {produced} of {requested} requested images. They are attached to your reply."
            )
        } else {
            format!("Generated {produced} image(s). They are attached to your reply.")
        };
        let attachments = result
            .images
            .into_iter()
            .enumerate()
            .map(|(i, image)| {
                let name = format!("image-{}.{}", i + 1, extension_for_mime(&image.mime));
                Attachment::new(name, image.mime, image.data.to_vec())
            })
            .collect();
        Ok(ToolOutput::success(content)
            .with_attachments(attachments)
            .with_data(json!({
                "images": produced,
                "requested": requested,
                "cost_usd": result.cost_usd,
            })))
    }
}

impl ImageGenerateHandler {
    /// Loads `reference_attachment_ids`: images attached in this conversation.
    async fn references(
        &self,
        params: &HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> std::result::Result<Vec<ReferenceImage>, String> {
        let ids: Vec<&str> = params
            .get("reference_attachment_ids")
            .and_then(Value::as_array)
            .map(|ids| ids.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if ids.len() > MAX_REFERENCES {
            return Err(format!(
                "image-generate: at most {MAX_REFERENCES} reference images, got {}",
                ids.len()
            ));
        }
        let mut references = Vec::with_capacity(ids.len());
        for raw in ids {
            let not_found =
                || format!("image-generate: attachment `{raw}` was not found in this conversation");
            let id = Uuid::parse_str(raw).map_err(|_| not_found())?;
            let meta = self
                .attachments
                .get_meta(id)
                .await
                .map_err(|_| not_found())?;
            if meta.conversation_id != ctx.conversation_id {
                return Err(not_found());
            }
            if !meta.mime_type.starts_with("image/") {
                return Err(format!(
                    "image-generate: attachment `{raw}` is {}, not an image",
                    meta.mime_type
                ));
            }
            let data =
                self.attachments.load_bytes(id).await.map_err(|e| {
                    format!("image-generate: could not read attachment `{raw}`: {e}")
                })?;
            references.push(ReferenceImage {
                mime: meta.mime_type,
                data: data.into(),
            });
        }
        Ok(references)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use serde_json::{Value, json};
    use uuid::Uuid;

    use assistant_core::types::conversation::{ExecutionContext, Interface};
    use assistant_core::{
        AttachmentMeta, GeneratedImage, ImageGenerationProvider, ImageRequest, ImageResult,
        ToolHandler,
    };
    use assistant_storage::{AttachmentStore, InMemoryAttachmentStore};

    use super::ImageGenerateHandler;

    /// Returns `produced` PNGs and records each request.
    struct FakeImages {
        produced: usize,
        references: bool,
        seen: Mutex<Vec<ImageRequest>>,
    }

    #[async_trait]
    impl ImageGenerationProvider for FakeImages {
        fn name(&self) -> &str {
            "fake"
        }
        fn accepts_references(&self) -> bool {
            self.references
        }
        async fn generate(&self, request: ImageRequest) -> anyhow::Result<ImageResult> {
            self.seen.lock().unwrap().push(request);
            Ok(ImageResult {
                images: (0..self.produced)
                    .map(|_| GeneratedImage {
                        mime: "image/png".into(),
                        data: bytes::Bytes::from_static(b"png"),
                    })
                    .collect(),
                cost_usd: Some(0.08),
            })
        }
    }

    fn fake(produced: usize) -> Arc<FakeImages> {
        Arc::new(FakeImages {
            produced,
            references: true,
            seen: Mutex::new(Vec::new()),
        })
    }

    fn ctx(conversation_id: Uuid) -> ExecutionContext {
        ExecutionContext {
            conversation_id,
            agent_id: "test".into(),
            turn: 0,
            interface: Interface::Web,
            interactive: true,
            run_id: None,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    fn params(value: Value) -> HashMap<String, Value> {
        serde_json::from_value(value).unwrap()
    }

    async fn store_attachment(
        store: &InMemoryAttachmentStore,
        conversation_id: Uuid,
        mime: &str,
    ) -> Uuid {
        let meta = AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id,
            agent_id: "test".into(),
            filename: "photo".into(),
            mime_type: mime.into(),
            size_bytes: 3,
            created_at: chrono::Utc::now(),
        };
        store.store(&meta, b"jpg").await.unwrap();
        meta.id
    }

    #[test]
    fn schema_requires_a_prompt_and_bounds_n_and_references() {
        let handler = ImageGenerateHandler::new(fake(1), Arc::new(InMemoryAttachmentStore::new()));
        let schema = handler.params_schema();
        assert_eq!(schema["required"], json!(["prompt"]));
        assert_eq!(schema["properties"]["n"]["minimum"], 1);
        assert_eq!(schema["properties"]["n"]["maximum"], 4);
        assert_eq!(
            schema["properties"]["reference_attachment_ids"]["maxItems"],
            16
        );
        for key in [
            "aspect_ratio",
            "resolution",
            "quality",
            "background",
            "seed",
        ] {
            assert!(schema["properties"].get(key).is_some(), "{key}");
        }
    }

    #[tokio::test]
    async fn returns_the_images_as_attachments_with_the_cost() {
        let provider = fake(2);
        let handler =
            ImageGenerateHandler::new(provider.clone(), Arc::new(InMemoryAttachmentStore::new()));

        let output = handler
            .run(
                params(json!({"prompt": "a lighthouse", "n": 2, "aspect_ratio": "16:9"})),
                &ctx(Uuid::new_v4()),
            )
            .await
            .unwrap();

        assert!(output.success, "{}", output.content);
        assert_eq!(output.attachments.len(), 2);
        assert_eq!(output.attachments[0].mime_type, "image/png");
        assert_eq!(output.data.unwrap()["cost_usd"], 0.08);
        let seen = provider.seen.lock().unwrap();
        assert_eq!(seen[0].n, Some(2));
        assert_eq!(seen[0].aspect_ratio.as_deref(), Some("16:9"));
    }

    #[tokio::test]
    async fn fewer_images_than_requested_are_reported() {
        let handler = ImageGenerateHandler::new(fake(1), Arc::new(InMemoryAttachmentStore::new()));

        let output = handler
            .run(
                params(json!({"prompt": "cats", "n": 3})),
                &ctx(Uuid::new_v4()),
            )
            .await
            .unwrap();

        assert!(output.content.contains("1 of 3"), "{}", output.content);
    }

    #[tokio::test]
    async fn references_are_loaded_from_the_conversation() {
        let provider = fake(1);
        let store = Arc::new(InMemoryAttachmentStore::new());
        let conversation = Uuid::new_v4();
        let photo = store_attachment(&store, conversation, "image/jpeg").await;
        let handler = ImageGenerateHandler::new(provider.clone(), store);

        let output = handler
            .run(
                params(json!({"prompt": "make the sky purple", "reference_attachment_ids": [photo.to_string()]})),
                &ctx(conversation),
            )
            .await
            .unwrap();

        assert!(output.success, "{}", output.content);
        let seen = provider.seen.lock().unwrap();
        assert_eq!(seen[0].references.len(), 1);
        assert_eq!(seen[0].references[0].mime, "image/jpeg");
        assert_eq!(&seen[0].references[0].data[..], b"jpg");
    }

    #[tokio::test]
    async fn non_images_other_conversations_and_too_many_references_are_rejected() {
        let provider = fake(1);
        let store = Arc::new(InMemoryAttachmentStore::new());
        let conversation = Uuid::new_v4();
        let pdf = store_attachment(&store, conversation, "application/pdf").await;
        let foreign = store_attachment(&store, Uuid::new_v4(), "image/png").await;
        let handler = ImageGenerateHandler::new(provider.clone(), store);

        for (ids, expect) in [
            (json!([pdf.to_string()]), "not an image"),
            (json!([foreign.to_string()]), "not found"),
            (
                json!(
                    (0..17)
                        .map(|_| Uuid::new_v4().to_string())
                        .collect::<Vec<_>>()
                ),
                "16",
            ),
        ] {
            let output = handler
                .run(
                    params(json!({"prompt": "edit", "reference_attachment_ids": ids})),
                    &ctx(conversation),
                )
                .await
                .unwrap();
            assert!(!output.success);
            assert!(
                output.content.contains(expect),
                "{expect}: {}",
                output.content
            );
        }
        assert!(
            provider.seen.lock().unwrap().is_empty(),
            "nothing was generated"
        );
    }

    #[tokio::test]
    async fn references_are_rejected_when_the_model_lacks_image_input() {
        let provider = Arc::new(FakeImages {
            produced: 1,
            references: false,
            seen: Mutex::new(Vec::new()),
        });
        let store = Arc::new(InMemoryAttachmentStore::new());
        let conversation = Uuid::new_v4();
        let photo = store_attachment(&store, conversation, "image/png").await;
        let handler = ImageGenerateHandler::new(provider, store);

        let output = handler
            .run(
                params(json!({"prompt": "edit", "reference_attachment_ids": [photo.to_string()]})),
                &ctx(conversation),
            )
            .await
            .unwrap();
        assert!(!output.success);
        assert!(output.content.contains("reference"), "{}", output.content);
    }

    #[tokio::test]
    async fn a_missing_prompt_is_an_error() {
        let handler = ImageGenerateHandler::new(fake(1), Arc::new(InMemoryAttachmentStore::new()));
        let output = handler
            .run(params(json!({})), &ctx(Uuid::new_v4()))
            .await
            .unwrap();
        assert!(!output.success);
        assert!(output.content.contains("prompt"), "{}", output.content);
    }
}
