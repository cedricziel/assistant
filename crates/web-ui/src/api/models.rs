//! `GET /api/models` — the models a member may pick to regenerate a reply
//! (ADR-0018, D10).

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use assistant_core::types::llm::LlmConfig;

use crate::space::{Access, SpaceCtx};

use super::ApiState;

/// One model offered for a regenerated reply.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModelSummary {
    /// The model name, as the provider knows it.
    pub name: String,
    /// Whether this is the org's default model.
    pub is_default: bool,
}

/// Whether `model` may answer a regenerate: the org's default model or one
/// of its `available_models`.
pub(crate) fn is_offered(llm: &LlmConfig, model: &str) -> bool {
    llm.model == model || llm.available_models.iter().any(|m| m == model)
}

/// `GET /api/models` — list the models a reply may be regenerated with.
#[utoipa::path(
    get,
    path = "/api/models",
    operation_id = "list_models",
    tag = "models",
    responses(
        (status = 200, description = "The default model first, then each listed model", body = Vec<ModelSummary>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_models(ctx: SpaceCtx, State(state): State<ApiState>) -> Response {
    if let Err(denied) = ctx.require(Access::Read) {
        return denied.into_response();
    }
    let runtime = match state.runtime(&ctx).await {
        Ok(runtime) => runtime,
        Err(response) => return response,
    };
    let llm = &runtime.config.llm;
    let listed = llm.available_models.iter().map(|name| (name, false));
    let models: Vec<ModelSummary> = std::iter::once((&llm.model, true))
        .chain(listed)
        .map(|(name, is_default)| ModelSummary {
            name: name.clone(),
            is_default,
        })
        .collect();
    Json(models).into_response()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use wiremock::MockServer;

    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{Role, SpaceId};

    use super::super::test_helpers::*;

    async fn get_models(auth: AuthContext, available: &[&str]) -> axum::response::Response {
        let server = MockServer::start().await;
        let (state, _) = models_state(&server.uri(), available).await;
        app_with_auth(state, auth)
            .oneshot(
                Request::builder()
                    .uri("/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn lists_the_default_first_then_the_listed_models() {
        let resp = get_models(AuthContext::system(), &["model-b", "model-c"]).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            body_json(resp.into_body()).await,
            serde_json::json!([
                { "name": "test", "is_default": true },
                { "name": "model-b", "is_default": false },
                { "name": "model-c", "is_default": false },
            ]),
            "a bare array, the default first"
        );
    }

    #[tokio::test]
    async fn lists_only_the_default_without_an_allowlist() {
        let resp = get_models(AuthContext::system(), &[]).await;

        assert_eq!(
            body_json(resp.into_body()).await,
            serde_json::json!([{ "name": "test", "is_default": true }])
        );
    }

    #[tokio::test]
    async fn a_read_only_member_may_list_models() {
        let mut viewer = AuthContext::system();
        viewer.space_roles =
            std::collections::HashMap::from([(SpaceId::from("default"), Role::Viewer)]);

        let resp = get_models(viewer, &["model-b"]).await;

        assert_eq!(resp.status(), StatusCode::OK);
    }
}
