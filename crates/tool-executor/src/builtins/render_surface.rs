//! `render-surface` tool — show the user a structured UI surface.
//!
//! The model describes a surface (title, fallback text, A2UI components, data
//! model); this handler validates it against the assistant catalog and the
//! org's URL policy and hands the validated surface back in `ToolOutput.data`.
//! The runtime's surface hook (not this handler) assigns the surface id,
//! persists it, merges updates and emits the event.
//!
//! Registered nowhere yet: `register_builtins` adds it once the clients that
//! render surfaces have shipped.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{debug, warn};
use uuid::Uuid;

use assistant_a2ui::{Catalog, SurfaceDraft, SurfacePatch, UrlPolicy, ValidationIssue, validate};
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_core::types::features::UiConfig;
use assistant_core::{ToolHandler, ToolOutput};
use assistant_storage::AttachmentStore;

/// Validates `render-surface` calls; see the module docs.
pub struct RenderSurfaceHandler {
    attachments: Arc<dyn AttachmentStore>,
    ui: UiConfig,
}

impl RenderSurfaceHandler {
    pub fn new(attachments: Arc<dyn AttachmentStore>, ui: UiConfig) -> Self {
        Self { attachments, ui }
    }

    /// Attachments among `ids` that belong to the turn's conversation.
    async fn owned_attachments(&self, ids: HashSet<Uuid>, conversation_id: Uuid) -> HashSet<Uuid> {
        let mut owned = HashSet::new();
        for id in ids {
            match self.attachments.get_meta(id).await {
                Ok(meta) if meta.conversation_id == conversation_id => {
                    owned.insert(id);
                }
                Ok(_) => debug!(%id, "render-surface: attachment belongs to another conversation"),
                Err(e) => debug!(%id, error = %e, "render-surface: attachment lookup failed"),
            }
        }
        owned
    }

    fn url_policy(&self, allowed_attachment_ids: HashSet<Uuid>) -> UrlPolicy {
        UrlPolicy {
            allowed_attachment_ids,
            image_hosts: (!self.ui.image_hosts.is_empty()).then(|| self.ui.image_hosts.clone()),
        }
    }

    async fn create(&self, params: &HashMap<String, Value>, ctx: &ExecutionContext) -> ToolOutput {
        let draft = match parse::<SurfaceDraft>(params) {
            Ok(d) => d,
            Err(issue) => return issues_output(&[issue]),
        };
        let owned = self
            .owned_attachments(draft.attachment_ids(), ctx.conversation_id)
            .await;
        match validate(&draft, &self.url_policy(owned)) {
            Ok(valid) => match serde_json::to_value(valid.draft()) {
                Ok(surface) => ToolOutput::success("Surface validated.")
                    .with_data(json!({ "surface": surface })),
                Err(e) => ToolOutput::error(format!("render-surface: cannot encode surface: {e}")),
            },
            Err(issues) => issues_output(&issues),
        }
    }

    async fn update(
        &self,
        surface_id: &str,
        params: &HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> ToolOutput {
        let patch = match parse::<SurfacePatch>(params) {
            Ok(p) => p,
            Err(issue) => return issues_output(&[issue]),
        };
        if patch.components.is_empty() && patch.data_updates.is_empty() {
            return ToolOutput::error(
                "render-surface: an update needs `components` or `data_updates`",
            );
        }
        let mut issues = Vec::new();
        if let Ok(catalog) = Catalog::embedded() {
            for (i, c) in patch.components.iter().enumerate() {
                for issue in catalog.validate_component(&c.to_value()) {
                    issues.push(ValidationIssue::new(
                        &issue.code,
                        format!("/components/{i}{}", issue.path),
                        issue.message,
                    ));
                }
            }
        }
        if !issues.is_empty() {
            return issues_output(&issues);
        }
        let owned = self
            .owned_attachments(patch.attachment_ids(), ctx.conversation_id)
            .await;
        let policy = self.url_policy(owned);
        let mut allowed: Vec<String> = policy
            .allowed_attachment_ids
            .iter()
            .map(Uuid::to_string)
            .collect();
        allowed.sort();
        let patch = match serde_json::to_value(&patch) {
            Ok(p) => p,
            Err(e) => {
                return ToolOutput::error(format!("render-surface: cannot encode patch: {e}"));
            }
        };
        // The merged surface can only be validated against the stored one, so
        // the surface hook does that with this policy.
        ToolOutput::success("Surface update validated.").with_data(json!({
            "surface_id": surface_id,
            "patch": patch,
            "url_policy": {
                "allowed_attachment_ids": allowed,
                "image_hosts": policy.image_hosts,
            },
        }))
    }
}

/// Deserialize the call's parameters, reporting a failure as one issue.
fn parse<T: serde::de::DeserializeOwned>(
    params: &HashMap<String, Value>,
) -> Result<T, ValidationIssue> {
    let object: serde_json::Map<String, Value> =
        params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    serde_json::from_value(Value::Object(object))
        .map_err(|e| ValidationIssue::new("PARAMS", "", e.to_string()))
}

/// An error output whose content is the JSON issue list.
fn issues_output(issues: &[ValidationIssue]) -> ToolOutput {
    match serde_json::to_string(issues) {
        Ok(json) => ToolOutput::error(json),
        Err(e) => {
            warn!(error = %e, "render-surface: cannot encode issues");
            ToolOutput::error("render-surface: the surface is invalid")
        }
    }
}

#[async_trait]
impl ToolHandler for RenderSurfaceHandler {
    fn name(&self) -> &str {
        "render-surface"
    }

    fn description(&self) -> &str {
        "Show the user a structured UI surface (cards, lists, forms, buttons) instead of plain text. \
         Create one with `title`, `fallback_text` and `components`; update one you created this turn \
         with `surface_id` plus `components` or `data_updates`. Components form a tree: exactly one \
         has id \"root\" and the others hang off it through `children`."
    }

    fn params_schema(&self) -> Value {
        let names = match Catalog::embedded() {
            Ok(catalog) => catalog.component_names(),
            Err(e) => {
                warn!(error = %e, "render-surface: catalog unavailable");
                Vec::new()
            }
        };
        json!({
            "type": "object",
            "properties": {
                "surface_id": {
                    "type": "string",
                    "description": "Update mode: the id of a surface created earlier in this turn. Omit to create."
                },
                "title": {
                    "type": "string",
                    "description": "Short heading. Required on create."
                },
                "fallback_text": {
                    "type": "string",
                    "description": "1-2000 characters of plain text that says what the surface shows, for clients that cannot render it. Required on create."
                },
                "components": {
                    "type": "array",
                    "description": "A2UI components, each with a unique `id`, a `component` type and that type's properties. On update they replace components with the same id and add new ones.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "string"},
                            "component": {"type": "string", "enum": names}
                        },
                        "required": ["id", "component"]
                    }
                },
                "data_model": {
                    "type": "object",
                    "description": "Initial values that components bind to with {\"path\": \"/pointer\"}."
                },
                "send_data_model": {
                    "type": "boolean",
                    "description": "Include the data model when the user answers (default true)."
                },
                "data_updates": {
                    "type": "array",
                    "description": "Update mode: values to set in the data model; a null value deletes the path.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "JSON Pointer, e.g. /name"},
                            "value": {}
                        },
                        "required": ["path", "value"]
                    }
                }
            },
            "required": []
        })
    }

    fn requires_rich_client(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        if !ctx.interactive || ctx.interface != Interface::Web {
            // A stale skill or a forced call; nothing can render the surface.
            let fallback = params
                .get("fallback_text")
                .and_then(Value::as_str)
                .unwrap_or("(no fallback text was given)");
            return Ok(ToolOutput::success(format!(
                "This interface cannot display UI; present the following to the user as text: {fallback}"
            )));
        }
        match params.get("surface_id").and_then(Value::as_str) {
            Some(id) if !id.is_empty() => Ok(self.update(id, &params, ctx).await),
            _ => Ok(self.create(&params, ctx).await),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use assistant_core::ToolHandler;
    use assistant_core::attachment::AttachmentMeta;
    use assistant_core::types::conversation::{ExecutionContext, Interface};
    use assistant_core::types::features::UiConfig;
    use assistant_storage::{AttachmentStore, InMemoryAttachmentStore};
    use chrono::Utc;
    use serde_json::{Value, json};
    use uuid::Uuid;

    use super::RenderSurfaceHandler;

    fn ctx(conversation_id: Uuid, interface: Interface, interactive: bool) -> ExecutionContext {
        ExecutionContext {
            conversation_id,
            agent_id: "test".to_string(),
            turn: 1,
            interface,
            interactive,
            run_id: None,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    fn web_ctx(conversation_id: Uuid) -> ExecutionContext {
        ctx(conversation_id, Interface::Web, true)
    }

    fn handler(store: Arc<InMemoryAttachmentStore>, ui: UiConfig) -> RenderSurfaceHandler {
        RenderSurfaceHandler::new(store, ui)
    }

    fn plain() -> RenderSurfaceHandler {
        handler(
            Arc::new(InMemoryAttachmentStore::new()),
            UiConfig::default(),
        )
    }

    fn params(v: Value) -> HashMap<String, Value> {
        serde_json::from_value(v).expect("object")
    }

    fn text_surface() -> Value {
        json!({
            "title": "Greeting",
            "fallback_text": "Hello there",
            "components": [
                {"id": "root", "component": "Column", "children": ["t"]},
                {"id": "t", "component": "Text", "text": "hi"}
            ]
        })
    }

    fn image_surface(url: &str) -> Value {
        json!({
            "title": "Pic",
            "fallback_text": "A picture",
            "components": [
                {"id": "root", "component": "Column", "children": ["i"]},
                {"id": "i", "component": "Image", "url": url}
            ]
        })
    }

    async fn store_attachment(store: &InMemoryAttachmentStore, conversation_id: Uuid) -> Uuid {
        let meta = AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id,
            agent_id: "test".to_string(),
            filename: "p.png".to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: 1,
            created_at: Utc::now(),
        };
        store.store(&meta, &[0]).await.expect("store");
        meta.id
    }

    #[tokio::test]
    async fn valid_create_returns_the_surface_as_data() {
        let out = plain()
            .run(params(text_surface()), &web_ctx(Uuid::new_v4()))
            .await
            .expect("run");
        assert!(out.success, "{}", out.content);
        let data = out.data.expect("data");
        assert_eq!(data["surface"]["title"], "Greeting");
        assert_eq!(
            data["surface"]["components"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(data["surface"]["send_data_model"], true, "defaults to true");
    }

    #[tokio::test]
    async fn invalid_draft_returns_the_issue_list_as_json() {
        let mut p = text_surface();
        p["components"][1]["component"] = json!("Video");
        let out = plain()
            .run(params(p), &web_ctx(Uuid::new_v4()))
            .await
            .expect("run");
        assert!(!out.success);
        assert!(out.data.is_none(), "a failed call carries no surface");
        let issues: Value = serde_json::from_str(&out.content).expect("content is the issue list");
        let first = &issues.as_array().expect("array")[0];
        assert!(
            first["code"].is_string() && first["path"].is_string(),
            "{issues}"
        );
    }

    #[tokio::test]
    async fn create_without_required_fields_fails() {
        let out = plain()
            .run(params(json!({"title": "x"})), &web_ctx(Uuid::new_v4()))
            .await
            .expect("run");
        assert!(!out.success, "{}", out.content);
    }

    #[tokio::test]
    async fn attachment_of_the_conversation_is_accepted() {
        let store = Arc::new(InMemoryAttachmentStore::new());
        let conv = Uuid::new_v4();
        let id = store_attachment(&store, conv).await;
        let h = handler(store, UiConfig::default());
        let out = h
            .run(
                params(image_surface(&format!("/api/attachments/{id}"))),
                &web_ctx(conv),
            )
            .await
            .expect("run");
        assert!(out.success, "{}", out.content);
    }

    #[tokio::test]
    async fn attachment_of_another_conversation_fails() {
        let store = Arc::new(InMemoryAttachmentStore::new());
        let id = store_attachment(&store, Uuid::new_v4()).await;
        let h = handler(store, UiConfig::default());
        let out = h
            .run(
                params(image_surface(&format!("/api/attachments/{id}"))),
                &web_ctx(Uuid::new_v4()),
            )
            .await
            .expect("run");
        assert!(!out.success);
        assert!(out.content.contains("URL_POLICY"), "{}", out.content);
    }

    #[tokio::test]
    async fn image_host_outside_the_list_fails_and_inside_passes() {
        let ui = UiConfig {
            image_hosts: vec!["images.example".to_string()],
            link_hosts: vec![],
        };
        let h = handler(Arc::new(InMemoryAttachmentStore::new()), ui);
        let conv = Uuid::new_v4();
        let bad = h
            .run(
                params(image_surface("https://tracker.example/p.png")),
                &web_ctx(conv),
            )
            .await
            .expect("run");
        assert!(!bad.success);
        assert!(bad.content.contains("tracker.example"), "{}", bad.content);
        let good = h
            .run(
                params(image_surface("https://images.example/p.png")),
                &web_ctx(conv),
            )
            .await
            .expect("run");
        assert!(good.success, "{}", good.content);
    }

    #[tokio::test]
    async fn any_https_image_host_is_allowed_without_a_list() {
        let out = plain()
            .run(
                params(image_surface("https://anywhere.example/p.png")),
                &web_ctx(Uuid::new_v4()),
            )
            .await
            .expect("run");
        assert!(out.success, "{}", out.content);
    }

    #[tokio::test]
    async fn update_without_components_or_data_updates_fails() {
        let out = plain()
            .run(
                params(json!({"surface_id": "s1", "title": "New title"})),
                &web_ctx(Uuid::new_v4()),
            )
            .await
            .expect("run");
        assert!(!out.success);
        assert!(out.content.contains("components"), "{}", out.content);
    }

    #[tokio::test]
    async fn update_returns_the_patch_and_url_policy_as_data() {
        let store = Arc::new(InMemoryAttachmentStore::new());
        let conv = Uuid::new_v4();
        let id = store_attachment(&store, conv).await;
        let ui = UiConfig {
            image_hosts: vec!["images.example".to_string()],
            link_hosts: vec![],
        };
        let h = handler(store, ui);
        let out = h
            .run(
                params(json!({
                    "surface_id": "s1",
                    "components": [{"id": "i", "component": "Image", "url": format!("/api/attachments/{id}")}],
                    "data_updates": [{"path": "/name", "value": "Ada"}]
                })),
                &web_ctx(conv),
            )
            .await
            .expect("run");
        assert!(out.success, "{}", out.content);
        let data = out.data.expect("data");
        assert_eq!(data["surface_id"], "s1");
        assert_eq!(data["patch"]["data_updates"][0]["path"], "/name");
        assert_eq!(data["url_policy"]["image_hosts"], json!(["images.example"]));
        assert_eq!(
            data["url_policy"]["allowed_attachment_ids"],
            json!([id.to_string()])
        );
        assert!(data.get("surface").is_none());
    }

    #[tokio::test]
    async fn update_policy_omits_attachments_of_other_conversations() {
        let store = Arc::new(InMemoryAttachmentStore::new());
        let id = store_attachment(&store, Uuid::new_v4()).await;
        let h = handler(store, UiConfig::default());
        let out = h
            .run(
                params(json!({
                    "surface_id": "s1",
                    "components": [{"id": "i", "component": "Image", "url": format!("/api/attachments/{id}")}]
                })),
                &web_ctx(Uuid::new_v4()),
            )
            .await
            .expect("run");
        assert!(out.success, "{}", out.content);
        let data = out.data.expect("data");
        assert_eq!(
            data["url_policy"]["allowed_attachment_ids"],
            json!([]),
            "the hook's merge then rejects the foreign attachment"
        );
    }

    #[tokio::test]
    async fn update_with_an_unknown_component_fails() {
        let out = plain()
            .run(
                params(json!({
                    "surface_id": "s1",
                    "components": [{"id": "v", "component": "Video"}]
                })),
                &web_ctx(Uuid::new_v4()),
            )
            .await
            .expect("run");
        assert!(!out.success);
        assert!(out.content.contains("/components/0"), "{}", out.content);
    }

    #[tokio::test]
    async fn outside_an_interactive_web_turn_returns_the_text_fallback() {
        let conv = Uuid::new_v4();
        for c in [
            ctx(conv, Interface::Web, false),
            ctx(conv, Interface::Matrix, true),
            ctx(conv, Interface::Cli, true),
        ] {
            let out = plain().run(params(text_surface()), &c).await.expect("run");
            assert!(out.success, "{}", out.content);
            assert!(
                out.data.is_none(),
                "nothing to render for {:?}",
                c.interface
            );
            assert!(out.content.contains("cannot display UI"), "{}", out.content);
            assert!(out.content.contains("Hello there"), "{}", out.content);
        }
    }

    #[tokio::test]
    async fn text_fallback_without_fallback_text_still_succeeds() {
        let out = plain()
            .run(
                params(json!({"title": "x"})),
                &ctx(Uuid::new_v4(), Interface::Slack, true),
            )
            .await
            .expect("run");
        assert!(out.success);
        assert!(out.data.is_none());
    }

    #[test]
    fn handler_is_a_rich_client_tool_named_render_surface() {
        let h = plain();
        assert_eq!(h.name(), "render-surface");
        assert!(h.requires_rich_client());
        assert!(!h.is_mutating());
    }

    #[test]
    fn params_schema_is_compact_and_constrains_components() {
        let schema = plain().params_schema();
        let size = serde_json::to_string(&schema).expect("serialize").len();
        assert!(size <= 6 * 1024, "schema is {size} bytes");
        let props = &schema["properties"];
        assert_eq!(props["components"]["type"], "array");
        assert_eq!(props["components"]["items"]["type"], "object");
        let names = props["components"]["items"]["properties"]["component"]["enum"]
            .as_array()
            .expect("component enum");
        assert!(names.contains(&json!("Text")) && names.contains(&json!("Button")));
        assert!(!names.contains(&json!("Video")));
        assert!(
            props.get("wait_for_action").is_none(),
            "wait_for_action lands with PR 20"
        );
    }
}
