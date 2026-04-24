//! REST JSON API for space membership management.
//!
//! ## Routes
//!
//! | Method | Path                                                     | Description       |
//! |--------|----------------------------------------------------------|-------------------|
//! | GET    | `/api/orgs/{org_id}/spaces/{space_id}/members`           | List members      |
//! | POST   | `/api/orgs/{org_id}/spaces/{space_id}/members`           | Add member        |
//! | PATCH  | `/api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` | Change role       |
//! | DELETE | `/api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` | Remove member     |

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use assistant_core::MembershipStore;
use assistant_core::auth::AuthContext;
use assistant_core::identity::{Role, SpaceId, UserId};
use assistant_core::store::SpaceMembership;
use assistant_storage::OrgStorageLayer;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct MembersApiState {
    pub org_storage: Arc<OrgStorageLayer>,
}

// -- Request/response types --------------------------------------------------

/// A space member entry.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MemberEntry {
    pub user_id: String,
    pub space_id: String,
    pub role: String,
    pub created_at: String,
}

/// Body for `POST /api/orgs/{org_id}/spaces/{space_id}/members`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AddMemberRequest {
    pub user_id: String,
    /// Role: `"org-admin"`, `"space-admin"`, `"member"`, or `"viewer"`.
    pub role: String,
}

/// Body for `PATCH /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateMemberRequest {
    /// New role for the member.
    pub role: String,
}

// -- Router ------------------------------------------------------------------

pub fn members_api_router() -> Router<MembersApiState> {
    Router::new()
        .route(
            "/orgs/{org_id}/spaces/{space_id}/members",
            get(list_members).post(add_member),
        )
        .route(
            "/orgs/{org_id}/spaces/{space_id}/members/{user_id}",
            axum::routing::patch(update_member).delete(remove_member),
        )
}

// -- Helpers -----------------------------------------------------------------

fn parse_role(s: &str) -> Option<Role> {
    match s {
        "org-admin" | "OrgAdmin" => Some(Role::OrgAdmin),
        "space-admin" | "SpaceAdmin" => Some(Role::SpaceAdmin),
        "member" | "Member" => Some(Role::Member),
        "viewer" | "Viewer" => Some(Role::Viewer),
        _ => None,
    }
}

fn role_to_str(role: &Role) -> &'static str {
    match role {
        Role::OrgAdmin => "org-admin",
        Role::SpaceAdmin => "space-admin",
        Role::Member => "member",
        Role::Viewer => "viewer",
    }
}

/// Check if the caller can manage members in this space.
fn can_manage_members(ctx: &AuthContext, space_id: &SpaceId) -> bool {
    if ctx.is_org_admin() {
        return true;
    }
    matches!(ctx.role_in(space_id), Some(Role::SpaceAdmin))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/orgs/{org_id}/spaces/{space_id}/members` — list space members.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/spaces/{space_id}/members",
    tag = "members",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    responses(
        (status = 200, description = "List of members", body = Vec<MemberEntry>),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn list_members(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<MembersApiState>,
    Path((_org_id, space_id)): Path<(String, String)>,
) -> Response {
    let space_id = SpaceId::from(space_id);

    if !ctx.is_org_admin() && ctx.role_in(&space_id).is_none() {
        return (StatusCode::FORBIDDEN, "access denied").into_response();
    }

    let store = state.org_storage.membership_store();
    match store.get_members_of_space(&space_id).await {
        Ok(members) => {
            let entries: Vec<MemberEntry> = members
                .into_iter()
                .map(|m| MemberEntry {
                    user_id: m.user_id.0,
                    space_id: m.space_id.0,
                    role: role_to_str(&m.role).into(),
                    created_at: m.created_at.to_rfc3339(),
                })
                .collect();
            Json(entries).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list members: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
        }
    }
}

/// `POST /api/orgs/{org_id}/spaces/{space_id}/members` — add a member.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/spaces/{space_id}/members",
    tag = "members",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    request_body = AddMemberRequest,
    responses(
        (status = 201, description = "Member added", body = MemberEntry),
        (status = 400, description = "Invalid role"),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn add_member(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<MembersApiState>,
    Path((_org_id, space_id)): Path<(String, String)>,
    Json(body): Json<AddMemberRequest>,
) -> Response {
    let space_id = SpaceId::from(space_id);

    if !can_manage_members(&ctx, &space_id) {
        return (StatusCode::FORBIDDEN, "space admin or org admin required").into_response();
    }

    let role = match parse_role(&body.role) {
        Some(r) => r,
        None => return (StatusCode::BAD_REQUEST, "invalid role").into_response(),
    };

    let now = Utc::now();
    let membership = SpaceMembership {
        user_id: UserId::from(body.user_id),
        space_id: space_id.clone(),
        role: role.clone(),
        created_at: now,
    };

    let store = state.org_storage.membership_store();
    if let Err(e) = store.add_membership(&membership).await {
        tracing::error!("failed to add member: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to add member").into_response();
    }

    let entry = MemberEntry {
        user_id: membership.user_id.0,
        space_id: membership.space_id.0,
        role: role_to_str(&role).into(),
        created_at: now.to_rfc3339(),
    };
    (StatusCode::CREATED, Json(entry)).into_response()
}

/// `PATCH /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` — change role.
#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/spaces/{space_id}/members/{user_id}",
    tag = "members",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
        ("user_id" = String, Path, description = "User ID"),
    ),
    request_body = UpdateMemberRequest,
    responses(
        (status = 200, description = "Role updated", body = MemberEntry),
        (status = 400, description = "Invalid role"),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn update_member(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<MembersApiState>,
    Path((_org_id, space_id, user_id)): Path<(String, String, String)>,
    Json(body): Json<UpdateMemberRequest>,
) -> Response {
    let space_id = SpaceId::from(space_id);
    let user_id = UserId::from(user_id);

    if !can_manage_members(&ctx, &space_id) {
        return (StatusCode::FORBIDDEN, "space admin or org admin required").into_response();
    }

    let role = match parse_role(&body.role) {
        Some(r) => r,
        None => return (StatusCode::BAD_REQUEST, "invalid role").into_response(),
    };

    let store = state.org_storage.membership_store();

    // Remove old membership and add new one with updated role.
    let _ = store.remove_membership(&user_id, &space_id).await;

    let now = Utc::now();
    let membership = SpaceMembership {
        user_id: user_id.clone(),
        space_id: space_id.clone(),
        role: role.clone(),
        created_at: now,
    };

    if let Err(e) = store.add_membership(&membership).await {
        tracing::error!("failed to update member role: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to update role").into_response();
    }

    let entry = MemberEntry {
        user_id: user_id.0,
        space_id: space_id.0,
        role: role_to_str(&role).into(),
        created_at: now.to_rfc3339(),
    };
    Json(entry).into_response()
}

/// `DELETE /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` — remove member.
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/spaces/{space_id}/members/{user_id}",
    tag = "members",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
        ("user_id" = String, Path, description = "User ID"),
    ),
    responses(
        (status = 204, description = "Member removed"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn remove_member(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<MembersApiState>,
    Path((_org_id, space_id, user_id)): Path<(String, String, String)>,
) -> Response {
    let space_id = SpaceId::from(space_id);
    let user_id = UserId::from(user_id);

    if !can_manage_members(&ctx, &space_id) {
        return (StatusCode::FORBIDDEN, "space admin or org admin required").into_response();
    }

    let store = state.org_storage.membership_store();
    match store.remove_membership(&user_id, &space_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "membership not found").into_response(),
        Err(e) => {
            tracing::error!("failed to remove member: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to remove member").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::collections::HashMap;
    use tower::ServiceExt;

    use assistant_core::identity::OrgId;
    use assistant_core::store::{Organization, Space, User};
    use assistant_core::{OrgStore, SpaceStore, UserStore};

    fn make_admin_ctx() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("default"), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: OrgId::from("org_1"),
            email: "admin@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    fn make_member_ctx() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        AuthContext {
            user_id: UserId::from("alice"),
            org_id: OrgId::from("org_1"),
            email: "alice@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    fn test_app(state: MembersApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(members_api_router().with_state(state))
            .layer(Extension(ctx))
    }

    async fn setup_state() -> MembersApiState {
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        let now = Utc::now();
        // Seed org, space, and users so FK constraints are satisfied.
        let org = Organization {
            id: OrgId::from("org_1"),
            name: "Test Org".into(),
            slug: "test".into(),
            auth_mode: "password".into(),
            created_at: now,
            updated_at: now,
        };
        org_storage.org_store().create_org(&org).await.unwrap();
        let space = Space {
            id: SpaceId::from("eng"),
            org_id: OrgId::from("org_1"),
            name: "Engineering".into(),
            slug: "eng".into(),
            created_at: now,
            updated_at: now,
        };
        org_storage
            .space_store()
            .create_space(&space)
            .await
            .unwrap();
        // Seed users referenced in membership tests.
        for (uid, email) in [("alice", "alice@test.local"), ("bob", "bob@test.local")] {
            let user = User {
                id: UserId::from(uid),
                org_id: OrgId::from("org_1"),
                email: email.into(),
                name: uid.into(),
                password_hash: String::new(),
                idp_issuer: None,
                idp_subject: None,
                created_at: now,
                updated_at: now,
            };
            org_storage.user_store().create_user(&user).await.unwrap();
        }
        MembersApiState { org_storage }
    }

    #[tokio::test]
    async fn add_and_list_members() {
        let state = setup_state().await;
        let app = test_app(state.clone(), make_admin_ctx());

        // Add
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/eng/members")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "user_id": "alice",
                            "role": "member"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // List
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/spaces/eng/members")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let members: Vec<MemberEntry> = serde_json::from_slice(&body).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].role, "member");
    }

    #[tokio::test]
    async fn member_cannot_add_members() {
        let state = setup_state().await;
        let app = test_app(state, make_member_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/eng/members")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "user_id": "bob",
                            "role": "viewer"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn update_member_role() {
        let state = setup_state().await;

        // Seed a membership.
        let membership = SpaceMembership {
            user_id: UserId::from("bob"),
            space_id: SpaceId::from("eng"),
            role: Role::Viewer,
            created_at: Utc::now(),
        };
        state
            .org_storage
            .membership_store()
            .add_membership(&membership)
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/orgs/org_1/spaces/eng/members/bob")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "role": "member"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let entry: MemberEntry = serde_json::from_slice(&body).unwrap();
        assert_eq!(entry.role, "member");
    }

    #[tokio::test]
    async fn remove_member() {
        let state = setup_state().await;

        // Seed a membership.
        let membership = SpaceMembership {
            user_id: UserId::from("bob"),
            space_id: SpaceId::from("eng"),
            role: Role::Member,
            created_at: Utc::now(),
        };
        state
            .org_storage
            .membership_store()
            .add_membership(&membership)
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/spaces/eng/members/bob")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
