//! `POST /api/integrations/{id}/test`: connect with the caller's credential.
//!
//! The connection is short-lived: it lists the server's tools, records the
//! outcome as the integration's health for the caller's subject, and is closed.
//! Turns use their own pooled connections in the orchestrator service.

use axum::{
    Extension, Json,
    extract::{Path, State},
};

use assistant_core::auth::AuthContext;
use assistant_core::integrations::{Availability, EffectiveIntegration, IntegrationLayer};
use assistant_mcp_client::ConnectError;

use super::IntegrationsApiState;
use super::error::{ApiError, ApiResult};
use super::manage::{caller_view, load_visible};
use super::types::TestIntegrationResponse;
use crate::openapi::ErrorBody;

/// `POST /api/integrations/{id}/test` — connect and list the tools.
///
/// Uses the credential the caller's turns would use: the shared one, or their
/// own for a per-user integration. A personal integration can be tested only
/// by its owner. A failed connection is still a `200` with `ok: false` and the
/// error, and marks the integration `unreachable` until a connection works.
#[utoipa::path(
    post,
    path = "/api/integrations/{id}/test",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("id" = String, Path, description = "Integration id, `int_<uuid>`")),
    responses(
        (status = 200, description = "The test ran; `ok` says whether it connected", body = TestIntegrationResponse),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "The caller may not use this integration's credential", body = ErrorBody),
        (status = 404, description = "No such integration, or the caller may not see it", body = ErrorBody),
        (status = 422, description = "The integration cannot be used by the caller now: disabled, blocked by policy, or no usable credential", body = ErrorBody),
    )
)]
pub async fn test_integration(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TestIntegrationResponse>> {
    let visible = load_visible(&state, &ctx, &id).await?;
    let integration = visible.integration;
    if integration.layer == IntegrationLayer::User && !visible.access.can_manage() {
        return Err(ApiError::forbidden(
            "only the owner can test a personal integration",
        ));
    }
    let policy = state.store.get_policy(&ctx.org_id).await?;
    let user = match integration.layer {
        IntegrationLayer::User => integration.owner_user_id.clone(),
        _ => Some(ctx.user_id.clone()),
    };
    let (availability, credential_subject) = state
        .effective
        .assess(&policy, &integration, user.as_ref())
        .await?;
    if availability != Availability::Available {
        return Err(ApiError::unprocessable(unusable_reason(availability)));
    }

    let outcome = state
        .connections
        .probe(&EffectiveIntegration {
            integration: integration.clone(),
            credential_subject,
        })
        .await;
    let (ok, tool_names, error) = match outcome {
        Ok(tools) => {
            let mut names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
            names.sort();
            (true, names, None)
        }
        Err(ConnectError::Internal(message)) => {
            return Err(anyhow::anyhow!(message).into());
        }
        Err(error) => (false, Vec::new(), Some(error.message().to_owned())),
    };

    let view = caller_view(&state, &ctx, &policy, &integration).await?;
    Ok(Json(TestIntegrationResponse {
        ok,
        tool_names,
        error,
        status: view.status,
        last_connected_at: view.last_connected_at,
        tool_count: view.tool_count,
    }))
}

fn unusable_reason(availability: Availability) -> &'static str {
    match availability {
        Availability::Available => "usable",
        Availability::Disabled => "the integration is disabled",
        Availability::BlockedByPolicy => "the org policy no longer permits this integration",
        Availability::NeedsConnection => "no credential is connected for you yet",
        Availability::AuthExpired => "your credential is no longer usable: set it again",
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use serde_json::{Value, json};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    use assistant_core::integrations::{CredentialMode, CredentialStatus, IntegrationLayer};

    use super::super::testing::{Harness, Who};

    const OK: StatusCode = StatusCode::OK;
    const FORBIDDEN: StatusCode = StatusCode::FORBIDDEN;
    const NOT_FOUND: StatusCode = StatusCode::NOT_FOUND;
    const UNPROCESSABLE: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;

    /// A scripted MCP server offering `tools`, or a `500` for everything.
    struct Mcp {
        tools: Vec<&'static str>,
        healthy: bool,
    }

    impl Respond for Mcp {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            if !self.healthy {
                return ResponseTemplate::new(500);
            }
            let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
            let id = body["id"].clone();
            let reply = |result: Value| {
                ResponseTemplate::new(200)
                    .insert_header("mcp-session-id", "s")
                    .set_body_json(json!({"jsonrpc": "2.0", "id": id, "result": result}))
            };
            match body["method"].as_str().unwrap_or_default() {
                "initialize" => reply(json!({
                    "protocolVersion": body["params"]["protocolVersion"],
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "s", "version": "1"},
                })),
                "tools/list" => reply(json!({"tools": self.tools.iter().map(|name| json!({
                    "name": name, "description": "d", "inputSchema": {"type": "object"},
                })).collect::<Vec<_>>()})),
                _ => ResponseTemplate::new(202),
            }
        }
    }

    async fn serve(tools: &[&'static str], healthy: bool) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(Mcp {
                tools: tools.to_vec(),
                healthy,
            })
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(405))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        server
    }

    fn uri(id: &str) -> String {
        format!("/api/integrations/{id}/test")
    }

    /// A shared space integration at `url` with a stored credential.
    async fn connected_space(h: &Harness, url: &str) {
        h.seed("int_space", IntegrationLayer::Space, None, |i| {
            i.connect_url = url.into();
        })
        .await;
        h.put(
            Who::EngAdmin,
            "/api/integrations/int_space/credential",
            json!({"value": "ghp_abc"}),
        )
        .await;
    }

    #[tokio::test]
    async fn a_working_connection_reports_the_tool_names() {
        let server = serve(&["list_issues", "create_issue"], true).await;
        let h = Harness::new().await;
        connected_space(&h, &server.uri()).await;

        let (status, body) = h.post(Who::EngAdmin, &uri("int_space"), json!({})).await;

        assert_eq!(status, OK, "{body}");
        assert_eq!(body["ok"], json!(true));
        assert_eq!(body["tool_names"], json!(["create_issue", "list_issues"]));
        assert_eq!(body["tool_count"], json!(2));
        assert!(body["last_connected_at"].is_string(), "{body}");
        assert_eq!(body["status"], json!("ready"));
        assert!(body.get("error").is_none() || body["error"].is_null());
    }

    #[tokio::test]
    async fn the_credential_goes_out_in_the_header_the_template_forms() {
        let server = serve(&["t"], true).await;
        let h = Harness::new().await;
        connected_space(&h, &server.uri()).await;

        h.post(Who::EngAdmin, &uri("int_space"), json!({})).await;

        let seen = server.received_requests().await.unwrap();
        assert!(
            seen.iter().any(
                |r| r.headers.get("authorization").and_then(|v| v.to_str().ok())
                    == Some("Bearer ghp_abc")
            ),
            "no request carried the credential"
        );
    }

    #[tokio::test]
    async fn the_result_is_what_the_list_then_shows() {
        let server = serve(&["a", "b", "c"], true).await;
        let h = Harness::new().await;
        connected_space(&h, &server.uri()).await;
        h.post(Who::EngAdmin, &uri("int_space"), json!({})).await;

        let (_, detail) = h.get(Who::Member, "/api/integrations/int_space").await;
        assert_eq!(detail["tool_count"], json!(3));
        assert!(detail["last_connected_at"].is_string(), "{detail}");

        let (_, list) = h.get(Who::Member, "/api/integrations").await;
        let item = &list["items"][0];
        assert_eq!(item["tool_count"], json!(3));
        assert_eq!(item["status"], json!("ready"));
    }

    #[tokio::test]
    async fn a_failed_connection_reports_the_error_and_marks_the_integration_unreachable() {
        let server = serve(&["t"], false).await;
        let h = Harness::new().await;
        connected_space(&h, &server.uri()).await;

        let (status, body) = h.post(Who::EngAdmin, &uri("int_space"), json!({})).await;

        assert_eq!(status, OK, "{body}");
        assert_eq!(body["ok"], json!(false));
        assert!(
            body["error"].as_str().is_some_and(|e| !e.is_empty()),
            "{body}"
        );
        assert_eq!(body["status"], json!("unreachable"));
        assert!(!body.to_string().contains("ghp_abc"), "{body}");

        let (_, detail) = h.get(Who::Member, "/api/integrations/int_space").await;
        assert_eq!(detail["status"], json!("unreachable"));
    }

    #[tokio::test]
    async fn a_later_success_clears_unreachable_and_keeps_the_last_good_connection() {
        let good = serve(&["a", "b"], true).await;
        let h = Harness::new().await;
        connected_space(&h, &good.uri()).await;
        h.post(Who::EngAdmin, &uri("int_space"), json!({})).await;

        // A turn finds the server down: the last good connection stays visible.
        h.state
            .health
            .record_failed("int_space", None, "down", super::super::testing::at(5))
            .await
            .unwrap();
        let (_, detail) = h.get(Who::Member, "/api/integrations/int_space").await;
        assert_eq!(detail["status"], json!("unreachable"));
        assert_eq!(detail["tool_count"], json!(2));

        let (_, body) = h.post(Who::EngAdmin, &uri("int_space"), json!({})).await;
        assert_eq!(body["status"], json!("ready"), "{body}");
    }

    #[tokio::test]
    async fn per_user_integrations_test_with_the_callers_own_credential() {
        let server = serve(&["t"], true).await;
        let h = Harness::new().await;
        h.seed("int_org", IntegrationLayer::Org, None, |i| {
            i.credential_mode = CredentialMode::PerUser;
            i.connect_url = server.uri();
        })
        .await;

        let (status, _) = h.post(Who::Member, &uri("int_org"), json!({})).await;
        assert_eq!(status, UNPROCESSABLE, "no credential connected yet");

        h.put(
            Who::Member,
            "/api/integrations/int_org/credential",
            json!({"value": "mine"}),
        )
        .await;
        let (status, body) = h.post(Who::Member, &uri("int_org"), json!({})).await;
        assert_eq!(status, OK, "{body}");
        assert_eq!(body["ok"], json!(true));
        let (status, _) = h.post(Who::Other, &uri("int_org"), json!({})).await;
        assert_eq!(status, UNPROCESSABLE, "another user has not connected");
    }

    #[tokio::test]
    async fn integrations_that_cannot_be_used_are_not_tested() {
        let server = serve(&["t"], true).await;
        let h = Harness::new().await;
        connected_space(&h, &server.uri()).await;
        h.patch(
            Who::EngAdmin,
            "/api/integrations/int_space",
            json!({"enabled": false}),
        )
        .await;

        let (status, _) = h.post(Who::EngAdmin, &uri("int_space"), json!({})).await;
        assert_eq!(status, UNPROCESSABLE);
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn who_may_test() {
        let server = serve(&["t"], true).await;
        let h = Harness::new().await;
        connected_space(&h, &server.uri()).await;
        h.seed("int_mine", IntegrationLayer::User, Some(Who::Member), |i| {
            i.connect_url = server.uri();
        })
        .await;
        h.seed_credential("int_mine", Some(Who::Member), CredentialStatus::Ready)
            .await;

        // A space member may test the shared space integration they use.
        let (status, _) = h.post(Who::Viewer, &uri("int_space"), json!({})).await;
        assert_eq!(status, OK);
        // Someone outside the space cannot see it.
        let (status, _) = h.post(Who::Other, &uri("int_space"), json!({})).await;
        assert_eq!(status, NOT_FOUND);
        // A personal integration is its owner's alone; an org admin may not use its credential.
        let (status, _) = h.post(Who::Admin, &uri("int_mine"), json!({})).await;
        assert_eq!(status, FORBIDDEN);
        let (status, _) = h.post(Who::Other, &uri("int_mine"), json!({})).await;
        assert_eq!(status, NOT_FOUND);
    }

    /// The pool records health under the subject `assess` returns, so the view
    /// must read it from the same one (an integration that needs no credential
    /// has none, even when per-user).
    #[tokio::test]
    async fn health_recorded_by_a_turn_shows_for_integrations_without_a_credential() {
        use assistant_core::integrations::IntegrationAuthKind;
        let h = Harness::new().await;
        h.seed("int_org", IntegrationLayer::Org, None, |i| {
            i.credential_mode = CredentialMode::PerUser;
            i.auth_kind = IntegrationAuthKind::None;
            i.header_name = None;
            i.header_value_prefix = None;
        })
        .await;
        h.seed("int_mine", IntegrationLayer::User, Some(Who::Member), |i| {
            i.auth_kind = IntegrationAuthKind::None;
            i.header_name = None;
            i.header_value_prefix = None;
        })
        .await;
        h.state
            .health
            .record_connected("int_mine", None, 4, super::super::testing::at(3))
            .await
            .unwrap();
        for id in ["int_org", "int_mine"] {
            h.state
                .health
                .record_failed(id, None, "down", super::super::testing::at(5))
                .await
                .unwrap();
        }

        let (_, org) = h.get(Who::Member, "/api/integrations/int_org").await;
        assert_eq!(org["status"], json!("unreachable"), "{org}");
        let (_, mine) = h.get(Who::Member, "/api/integrations/int_mine").await;
        assert_eq!(mine["tool_count"], json!(4), "{mine}");
        assert_eq!(mine["status"], json!("unreachable"), "{mine}");
    }

    #[tokio::test]
    async fn an_integration_that_needs_no_credential_can_be_tested() {
        let server = serve(&["t"], true).await;
        let h = Harness::new().await;
        h.seed("int_open", IntegrationLayer::Space, None, |i| {
            i.connect_url = server.uri();
            i.auth_kind = assistant_core::integrations::IntegrationAuthKind::None;
            i.header_name = None;
            i.header_value_prefix = None;
        })
        .await;
        let (status, body) = h.post(Who::EngAdmin, &uri("int_open"), json!({})).await;
        assert_eq!(status, OK, "{body}");
        assert_eq!(body["ok"], json!(true));
    }
}
