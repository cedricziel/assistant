//! `/api/integrations/{id}/spaces/{space_id}`: which spaces use an org integration.
//!
//! An org-layer integration is offered to every space unless an org admin
//! lists spaces with `allow` rows. A space admin may `opt_out` their own space
//! and remove that row to opt back in. The two kinds of row are independent.

use axum::{
    Extension, Json,
    extract::{
        Path, Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::StatusCode,
    response::Response,
};

use assistant_core::auth::AuthContext;
use assistant_core::identity::SpaceId;
use assistant_core::integrations::{IntegrationLayer, SpaceAccessMode};

use super::IntegrationsApiState;
use super::authz;
use super::error::{ApiError, ApiResult};
use super::manage::{into_response, load_visible};
use super::types::{DeleteSpaceAccessQuery, SetSpaceAccessRequest, SpaceAccessEntry};
use crate::errors::json_error;
use crate::openapi::ErrorBody;

/// `PUT /api/integrations/{id}/spaces/{space_id}` — add a space row.
///
/// `mode: allow` (org admins) restricts the integration to the spaces that have
/// an allow row. `mode: opt_out` (admins of that space, or org admins) removes
/// it from that space.
#[utoipa::path(
    put,
    path = "/api/integrations/{id}/spaces/{space_id}",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(
        ("id" = String, Path, description = "Org-layer integration id, `int_<uuid>`"),
        ("space_id" = String, Path, description = "Space id"),
    ),
    request_body = SetSpaceAccessRequest,
    responses(
        (status = 200, description = "The row now in place (idempotent)", body = SpaceAccessEntry),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "The caller may not set this row", body = ErrorBody),
        (status = 404, description = "No such integration or space, or the caller may not see the integration", body = ErrorBody),
        (status = 422, description = "Not an org-layer integration, or an invalid mode", body = ErrorBody),
    )
)]
pub async fn put_space_access(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path((id, space_id)): Path<(String, String)>,
    body: Result<Json<SetSpaceAccessRequest>, JsonRejection>,
) -> Response {
    let result = async {
        let Json(request) = body?;
        let mode = request.mode.into();
        let space = authorize(&state, &ctx, &id, &space_id, mode).await?;
        state.store.add_space_access(&id, &space, mode).await?;
        Ok(Json(SpaceAccessEntry {
            space_id: space.0,
            mode: mode.into(),
        }))
    }
    .await;
    into_response(result)
}

/// `DELETE /api/integrations/{id}/spaces/{space_id}?mode=` — remove a space row.
///
/// The `mode` query parameter names the row (`allow` or `opt_out`) and has the
/// same permissions as `PUT`. Removing a row that does not exist succeeds.
#[utoipa::path(
    delete,
    path = "/api/integrations/{id}/spaces/{space_id}",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(
        ("id" = String, Path, description = "Org-layer integration id, `int_<uuid>`"),
        ("space_id" = String, Path, description = "Space id"),
        DeleteSpaceAccessQuery,
    ),
    responses(
        (status = 204, description = "The row is gone"),
        (status = 400, description = "Missing or invalid `mode`", body = ErrorBody),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "The caller may not remove this row", body = ErrorBody),
        (status = 404, description = "No such integration or space, or the caller may not see the integration", body = ErrorBody),
        (status = 422, description = "Not an org-layer integration", body = ErrorBody),
    )
)]
pub async fn delete_space_access(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path((id, space_id)): Path<(String, String)>,
    query: Result<Query<DeleteSpaceAccessQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(rejection) => return json_error(StatusCode::BAD_REQUEST, rejection.body_text()),
    };
    let result = async {
        let mode = query.mode.into();
        let space = authorize(&state, &ctx, &id, &space_id, mode).await?;
        state.store.remove_space_access(&id, &space, mode).await?;
        Ok(StatusCode::NO_CONTENT)
    }
    .await;
    into_response(result)
}

/// Check the caller may set or remove a `mode` row of `space_id` on
/// integration `id`, and that both exist.
async fn authorize(
    state: &IntegrationsApiState,
    ctx: &AuthContext,
    id: &str,
    space_id: &str,
    mode: SpaceAccessMode,
) -> ApiResult<SpaceId> {
    let visible = load_visible(state, ctx, id).await?;
    if visible.integration.layer != IntegrationLayer::Org {
        return Err(ApiError::unprocessable(
            "only org-layer integrations have space availability",
        ));
    }
    let space = SpaceId::from(space_id);
    let allowed = match mode {
        SpaceAccessMode::Allow => ctx.is_org_admin(),
        SpaceAccessMode::OptOut => authz::can_administer_space(ctx, &space),
    };
    if !allowed {
        return Err(ApiError::forbidden(match mode {
            SpaceAccessMode::Allow => "org admin required",
            SpaceAccessMode::OptOut => "space admin or org admin required",
        }));
    }
    let found = state.spaces.get_space(&space).await?;
    if found.is_none_or(|found| found.org_id != ctx.org_id) {
        return Err(ApiError::not_found("space not found"));
    }
    Ok(space)
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use serde_json::json;

    use assistant_core::integrations::{IntegrationLayer, SpaceAccess, SpaceAccessMode};

    use super::super::testing::{Harness, Who, eng};

    const OK: StatusCode = StatusCode::OK;
    const FORBIDDEN: StatusCode = StatusCode::FORBIDDEN;
    const NOT_FOUND: StatusCode = StatusCode::NOT_FOUND;
    const NO_CONTENT: StatusCode = StatusCode::NO_CONTENT;
    const UNPROCESSABLE: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;

    async fn world() -> Harness {
        let h = Harness::new().await;
        h.seed("int_org", IntegrationLayer::Org, None, |_| {}).await;
        h.seed("int_space", IntegrationLayer::Space, None, |_| {})
            .await;
        h.seed(
            "int_member",
            IntegrationLayer::User,
            Some(Who::Member),
            |_| {},
        )
        .await;
        h
    }

    async fn rows(h: &Harness) -> Vec<SpaceAccess> {
        h.state.store.list_space_access("int_org").await.unwrap()
    }

    fn row(space: &str, mode: SpaceAccessMode) -> SpaceAccess {
        SpaceAccess {
            integration_id: "int_org".into(),
            space_id: space.into(),
            mode,
        }
    }

    #[tokio::test]
    async fn an_org_admin_restricts_an_integration_to_listed_spaces() {
        let h = world().await;

        let (status, body) = h
            .put(
                Who::Admin,
                "/api/integrations/int_org/spaces/sp_sales",
                json!({"mode": "allow"}),
            )
            .await;

        assert_eq!(status, OK, "{body}");
        assert_eq!(body, json!({"space_id": "sp_sales", "mode": "allow"}));
        assert_eq!(rows(&h).await, [row("sp_sales", SpaceAccessMode::Allow)]);
        let (eng_view, _) = h.get(Who::Member, "/api/integrations/int_org").await;
        let (_, sales_view) = h.get(Who::Other, "/api/integrations/int_org").await;
        assert_eq!(eng_view, NOT_FOUND, "hidden from eng");
        assert_eq!(sales_view["space_access"][0]["space_id"], "sp_sales");
    }

    #[tokio::test]
    async fn only_org_admins_manage_allow_rows() {
        for who in [
            Who::EngAdmin,
            Who::Member,
            Who::Viewer,
            Who::Other,
            Who::SalesAdmin,
        ] {
            let h = world().await;
            let (put, _) = h
                .put(
                    who,
                    "/api/integrations/int_org/spaces/sp_eng",
                    json!({"mode": "allow"}),
                )
                .await;
            for space in [eng(), "sp_sales".into()] {
                h.state
                    .store
                    .add_space_access("int_org", &space, SpaceAccessMode::Allow)
                    .await
                    .unwrap();
            }
            let (delete, _) = h
                .delete(who, "/api/integrations/int_org/spaces/sp_eng?mode=allow")
                .await;
            assert_eq!(put, FORBIDDEN, "{who:?} PUT allow");
            assert_eq!(delete, FORBIDDEN, "{who:?} DELETE allow");
        }
    }

    #[tokio::test]
    async fn a_space_admin_opts_their_space_out_and_back_in() {
        let h = world().await;

        let (status, body) = h
            .put(
                Who::EngAdmin,
                "/api/integrations/int_org/spaces/sp_eng",
                json!({"mode": "opt_out"}),
            )
            .await;
        assert_eq!(status, OK, "{body}");
        assert_eq!(body, json!({"space_id": "sp_eng", "mode": "opt_out"}));
        assert_eq!(rows(&h).await, [row("sp_eng", SpaceAccessMode::OptOut)]);

        let (status, _) = h
            .delete(
                Who::EngAdmin,
                "/api/integrations/int_org/spaces/sp_eng?mode=opt_out",
            )
            .await;
        assert_eq!(status, NO_CONTENT);
        assert!(rows(&h).await.is_empty());
    }

    #[tokio::test]
    async fn opting_out_is_limited_to_admins_of_that_space() {
        let cases = [
            (Who::Admin, "sp_eng", OK),
            (Who::EngAdmin, "sp_eng", OK),
            (Who::EngAdmin, "sp_sales", FORBIDDEN),
            (Who::SalesAdmin, "sp_eng", FORBIDDEN),
            (Who::Member, "sp_eng", FORBIDDEN),
            (Who::Viewer, "sp_eng", FORBIDDEN),
            (Who::Other, "sp_sales", FORBIDDEN),
        ];
        for (who, space, want) in cases {
            let h = world().await;
            let (status, body) = h
                .put(
                    who,
                    &format!("/api/integrations/int_org/spaces/{space}"),
                    json!({"mode": "opt_out"}),
                )
                .await;
            assert_eq!(status, want, "{who:?} opting {space} out: {body}");
        }
    }

    #[tokio::test]
    async fn opting_out_keeps_the_allow_row() {
        let h = world().await;
        h.state
            .store
            .add_space_access("int_org", &eng(), SpaceAccessMode::Allow)
            .await
            .unwrap();

        h.put(
            Who::EngAdmin,
            "/api/integrations/int_org/spaces/sp_eng",
            json!({"mode": "opt_out"}),
        )
        .await;
        h.delete(
            Who::EngAdmin,
            "/api/integrations/int_org/spaces/sp_eng?mode=opt_out",
        )
        .await;

        assert_eq!(rows(&h).await, [row("sp_eng", SpaceAccessMode::Allow)]);
    }

    #[tokio::test]
    async fn deleting_a_missing_row_is_a_no_op() {
        let h = world().await;

        let (status, _) = h
            .delete(
                Who::Admin,
                "/api/integrations/int_org/spaces/sp_eng?mode=allow",
            )
            .await;

        assert_eq!(status, NO_CONTENT);
    }

    #[tokio::test]
    async fn only_org_integrations_have_space_availability() {
        let h = world().await;

        for id in ["int_space", "int_member"] {
            let (status, body) = h
                .put(
                    Who::Admin,
                    &format!("/api/integrations/{id}/spaces/sp_eng"),
                    json!({"mode": "opt_out"}),
                )
                .await;
            assert_eq!(status, UNPROCESSABLE, "{id}: {body}");
        }
    }

    #[tokio::test]
    async fn unknown_integrations_and_spaces_are_not_found() {
        let h = world().await;

        let (nope, _) = h
            .put(
                Who::Admin,
                "/api/integrations/int_nope/spaces/sp_eng",
                json!({"mode": "allow"}),
            )
            .await;
        let (no_space, _) = h
            .put(
                Who::Admin,
                "/api/integrations/int_org/spaces/sp_nope",
                json!({"mode": "allow"}),
            )
            .await;
        let (hidden, _) = h
            .put(
                Who::Other,
                "/api/integrations/int_member/spaces/sp_sales",
                json!({"mode": "opt_out"}),
            )
            .await;

        assert_eq!((nope, no_space, hidden), (NOT_FOUND, NOT_FOUND, NOT_FOUND));
    }

    #[tokio::test]
    async fn the_mode_is_required_and_validated() {
        let h = world().await;

        let (put, _) = h
            .put(
                Who::Admin,
                "/api/integrations/int_org/spaces/sp_eng",
                json!({"mode": "maybe"}),
            )
            .await;
        let (delete, body) = h
            .delete(Who::Admin, "/api/integrations/int_org/spaces/sp_eng")
            .await;

        assert_eq!(put, UNPROCESSABLE);
        assert_eq!(delete, StatusCode::BAD_REQUEST, "{body}");
        assert!(body["error"].is_string(), "{body}");
    }
}
