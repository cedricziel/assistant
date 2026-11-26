//! `GET /api/capabilities` — report server capability flags.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::space::{Access, SpaceCtx};

use super::ApiState;

/// Server capability flags.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ServerCapabilities {
    /// Whether the server can accept voice messages (STT configured).
    pub voice_send: bool,
    /// Whether the server can serve TTS audio for messages (TTS configured).
    pub voice_receive: bool,
    /// Generative-UI settings of the selected space's org.
    pub ui: UiCapabilities,
}

/// The part of the org's `[ui]` settings a client needs.
///
/// `image_hosts` is deliberately absent: the server enforces it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UiCapabilities {
    /// Lower-case hosts whose `openUrl` targets open without a confirmation
    /// prompt. Empty means every link asks first.
    pub link_hosts: Vec<String>,
}

/// `GET /api/capabilities` — return server capability flags.
#[utoipa::path(
    get,
    path = "/api/capabilities",
    tag = "capabilities",
    responses(
        (status = 200, description = "Server capabilities", body = ServerCapabilities),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_capabilities(ctx: SpaceCtx, State(state): State<ApiState>) -> Response {
    if let Err(denied) = ctx.require(Access::Read) {
        return denied.into_response();
    }
    let runtime = match state.runtime(&ctx).await {
        Ok(runtime) => runtime,
        Err(response) => return response,
    };
    let link_hosts = runtime
        .config
        .ui
        .as_ref()
        .map(|ui| ui.link_hosts.clone())
        .unwrap_or_default();
    Json(ServerCapabilities {
        voice_send: state.transcription_provider.is_some(),
        voice_receive: state.tts_provider.is_some(),
        ui: UiCapabilities { link_hosts },
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use wiremock::MockServer;

    use super::super::ApiState;
    use super::super::test_helpers::*;

    #[tokio::test]
    async fn capabilities_no_providers_returns_false() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        // Both providers absent — both flags must be false.
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/capabilities")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["voice_send"], false);
        assert_eq!(json["voice_receive"], false);
    }

    async fn link_hosts_of(config: assistant_core::types::agent::OrgConfig) -> serde_json::Value {
        let server = MockServer::start().await;
        let storage = std::sync::Arc::new(
            assistant_storage::StorageLayer::new_in_memory()
                .await
                .unwrap(),
        );
        let (state, _) = state_over_config(
            std::path::Path::new("/nonexistent/assistant-test-root"),
            &server.uri(),
            &[default_space(&storage)],
            false,
            config,
        )
        .await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/capabilities")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        body_json(resp.into_body()).await
    }

    #[tokio::test]
    async fn capabilities_expose_the_org_link_hosts() {
        let mut config = assistant_core::types::agent::OrgConfig::default();
        config.memory.enabled = false;
        config.ui = Some(assistant_core::types::features::UiConfig {
            image_hosts: vec!["img.example".into()],
            link_hosts: vec!["docs.example".into()],
        });
        let json = link_hosts_of(config).await;
        assert_eq!(
            json["ui"]["link_hosts"],
            serde_json::json!(["docs.example"]),
            "the org's link_hosts must be returned"
        );
        assert!(
            json["ui"].get("image_hosts").is_none(),
            "image_hosts is enforced by the server and must not be exposed"
        );
    }

    #[tokio::test]
    async fn capabilities_without_ui_section_have_empty_link_hosts() {
        let mut config = assistant_core::types::agent::OrgConfig::default();
        config.memory.enabled = false;
        let json = link_hosts_of(config).await;
        assert_eq!(json["ui"]["link_hosts"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn capabilities_return_only_the_selected_orgs_link_hosts() {
        use assistant_core::auth::AuthContext;
        use assistant_core::identity::{OrgId, SpaceId};
        use assistant_runtime::{OrgContexts, SpaceRuntimes};

        let root = tempfile::tempdir().unwrap();
        for (org, hosts) in [
            ("acme", "[\"acme.example\"]"),
            ("globex", "[\"globex.example\"]"),
        ] {
            let dir = root.path().join("orgs").join(org);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("org.toml"),
                format!("[ui]\nlink_hosts = {hosts}\n"),
            )
            .unwrap();
        }
        let space = |id, org_id, org_slug, storage| TestSpace {
            id,
            org_id,
            org_slug,
            slug: "eng",
            storage,
        };
        let new_storage = || async {
            std::sync::Arc::new(
                assistant_storage::StorageLayer::new_in_memory()
                    .await
                    .unwrap(),
            )
        };
        let spaces = [
            space("spc_a", "org_a", "acme", new_storage().await),
            space("spc_b", "org_b", "globex", new_storage().await),
        ];
        let resolver = resolver_in(root.path(), &spaces).await;
        let runtimes = std::sync::Arc::new(SpaceRuntimes::new(
            resolver.pools().clone(),
            std::sync::Arc::new(OrgContexts::from_files(root.path())),
            None,
        ));
        let state = ApiState::new(
            resolver,
            runtimes,
            std::sync::Arc::new(tokio::sync::RwLock::new("default".to_string())),
        );
        for (space_id, org_id, expected) in [
            ("spc_a", "org_a", "acme.example"),
            ("spc_b", "org_b", "globex.example"),
        ] {
            let auth = AuthContext::system_in(OrgId::from(org_id), SpaceId::from(space_id));
            let resp = app_with_auth(state.clone(), auth)
                .oneshot(
                    Request::builder()
                        .uri("/capabilities")
                        .header("x-space-id", space_id)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let json = body_json(resp.into_body()).await;
            assert_eq!(json["ui"]["link_hosts"], serde_json::json!([expected]));
        }
    }
}
