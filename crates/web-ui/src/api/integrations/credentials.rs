//! `/api/integrations/{id}/credential`: set and clear a header credential.
//!
//! Whose credential it is follows the credential mode: the integration itself
//! for `shared`, the caller for `per_user`, and the owner for a personal
//! integration. The value is sealed here and never read back by the web
//! process.

use axum::{
    Extension, Json,
    extract::{Path, State, rejection::JsonRejection},
    http::HeaderValue,
};

use assistant_core::auth::AuthContext;
use assistant_core::identity::UserId;
use assistant_core::integrations::{
    CredentialKind, CredentialMode, CredentialStatus, Integration, IntegrationAuthKind,
    IntegrationLayer, StoredCredential, credential_aad,
};
use assistant_core::secret::Secret;

use super::IntegrationsApiState;
use super::error::{ApiError, ApiResult};
use super::manage::{Visible, load_visible, validate_header};
use super::types::{CredentialResponse, SetCredentialRequest};
use crate::openapi::ErrorBody;

/// The longest credential accepted; real tokens are far shorter.
const MAX_VALUE_LEN: usize = 4096;

/// `PUT /api/integrations/{id}/credential` — store the header credential.
///
/// For a `shared` integration the credential is the integration's own and
/// needs someone who manages its layer. For a `per_user` one it is the
/// caller's, and anyone who can see the integration may set theirs. A personal
/// integration's credential is its owner's alone.
#[utoipa::path(
    put,
    path = "/api/integrations/{id}/credential",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("id" = String, Path, description = "Integration id, `int_<uuid>`")),
    request_body = SetCredentialRequest,
    responses(
        (status = 200, description = "Stored. Only whether a credential exists is reported", body = CredentialResponse),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "The caller may not set this credential", body = ErrorBody),
        (status = 404, description = "No such integration, or the caller may not see it", body = ErrorBody),
        (status = 422, description = "The integration takes no header credential, or the value or header name is invalid", body = ErrorBody),
    )
)]
pub async fn set_credential(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path(id): Path<String>,
    body: Result<Json<SetCredentialRequest>, JsonRejection>,
) -> ApiResult<Json<CredentialResponse>> {
    let Json(request) = body?;
    let visible = load_visible(&state, &ctx, &id).await?;
    let subject = subject_for(&ctx, &visible)?;
    let mut integration = visible.integration;

    let value = Secret::new(request.value.expose().trim());
    validate_value(&integration, &value)?;
    let now = state.clock.now();

    if let Some(name) = request.header_name {
        if integration.header_name.is_some() {
            return Err(ApiError::unprocessable(
                "the header name is fixed for this integration",
            ));
        }
        if !visible.access.can_manage() {
            return Err(ApiError::forbidden(
                "only someone who manages this integration can set its header name",
            ));
        }
        validate_header(&name, integration.header_value_prefix.as_deref())?;
        let read_at = integration.updated_at;
        integration.header_name = Some(name);
        integration.updated_at = now;
        if !state
            .store
            .update_if_unchanged(&integration, read_at)
            .await?
        {
            return Err(ApiError::not_found("integration not found"));
        }
    } else if integration.header_name.is_none() {
        return Err(ApiError::unprocessable(
            "this integration has no header name yet: send header_name with the value",
        ));
    }

    let aad = credential_aad(
        &integration.id,
        subject.as_ref().map(|user| user.0.as_str()),
        CredentialKind::Header.as_str(),
    );
    let sealed = state.secrets.seal(&value, aad.as_bytes())?;
    state
        .credentials
        .put(&StoredCredential {
            integration_id: integration.id,
            subject_user_id: subject,
            kind: CredentialKind::Header,
            ciphertext: sealed.ciphertext,
            nonce: sealed.nonce,
            key_id: sealed.key_id,
            expires_at: None,
            status: CredentialStatus::Ready,
            updated_at: now,
        })
        .await?;
    Ok(Json(CredentialResponse {
        has_credential: true,
        updated_at: Some(now),
    }))
}

/// `DELETE /api/integrations/{id}/credential` — remove the header credential.
///
/// Same permissions as setting it.
#[utoipa::path(
    delete,
    path = "/api/integrations/{id}/credential",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("id" = String, Path, description = "Integration id, `int_<uuid>`")),
    responses(
        (status = 200, description = "Removed; `has_credential` is false", body = CredentialResponse),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "The caller may not clear this credential", body = ErrorBody),
        (status = 404, description = "No such integration, or the caller may not see it", body = ErrorBody),
        (status = 422, description = "The integration takes no header credential", body = ErrorBody),
    )
)]
pub async fn clear_credential(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CredentialResponse>> {
    let visible = load_visible(&state, &ctx, &id).await?;
    let subject = subject_for(&ctx, &visible)?;
    state.credentials.delete(&id, subject.as_ref()).await?;
    Ok(Json(CredentialResponse {
        has_credential: false,
        updated_at: None,
    }))
}

/// Whose credential the caller is writing, or why they may not.
///
/// `None` is the integration's shared credential.
fn subject_for(ctx: &AuthContext, visible: &Visible) -> ApiResult<Option<UserId>> {
    let integration = &visible.integration;
    if !matches!(
        integration.auth_kind,
        IntegrationAuthKind::Header | IntegrationAuthKind::Mixed
    ) {
        return Err(ApiError::unprocessable(
            "this integration does not use a header credential",
        ));
    }
    let require_manage = |message: &str| {
        if visible.access.can_manage() {
            Ok(())
        } else {
            Err(ApiError::forbidden(message))
        }
    };
    match (integration.layer, integration.credential_mode) {
        (IntegrationLayer::User, _) => {
            require_manage("only the owner can change a personal integration's credential")?;
            Ok(integration.owner_user_id.clone())
        }
        (_, CredentialMode::Shared) => {
            require_manage(
                "only someone who manages this integration can change its shared credential",
            )?;
            Ok(None)
        }
        (_, CredentialMode::PerUser) => Ok(Some(ctx.user_id.clone())),
    }
}

/// Reject a value that cannot be sent as part of a header. The message never
/// repeats the value.
fn validate_value(integration: &Integration, value: &Secret) -> ApiResult<()> {
    if value.is_blank() {
        return Err(ApiError::unprocessable("value must not be empty"));
    }
    if value.expose().len() > MAX_VALUE_LEN {
        return Err(ApiError::unprocessable(format!(
            "value must be at most {MAX_VALUE_LEN} bytes"
        )));
    }
    let header = format!(
        "{}{}",
        integration.header_value_prefix.as_deref().unwrap_or(""),
        value.expose()
    );
    if HeaderValue::from_str(&header).is_err() {
        return Err(ApiError::unprocessable(
            "value contains characters that cannot be sent in an HTTP header",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use serde_json::{Value, json};

    use assistant_core::integrations::{
        CredentialKind, CredentialMode, IntegrationAuthKind, IntegrationLayer, Sealed,
        credential_aad,
    };

    use super::super::testing::{Harness, Who};

    const OK: StatusCode = StatusCode::OK;
    const FORBIDDEN: StatusCode = StatusCode::FORBIDDEN;
    const NOT_FOUND: StatusCode = StatusCode::NOT_FOUND;
    const UNPROCESSABLE: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;

    fn uri(id: &str) -> String {
        format!("/api/integrations/{id}/credential")
    }

    fn value(v: &str) -> Value {
        json!({ "value": v })
    }

    /// Decrypt what is stored for `(id, subject)`.
    async fn stored_plaintext(h: &Harness, id: &str, subject: Option<Who>) -> Option<String> {
        let stored = h
            .state
            .credentials
            .get(id, subject.map(Who::user).as_ref())
            .await
            .unwrap()?;
        let aad = credential_aad(
            id,
            subject.map(Who::user).as_ref().map(|u| u.0.as_str()),
            CredentialKind::Header.as_str(),
        );
        let sealed = Sealed {
            ciphertext: stored.ciphertext,
            nonce: stored.nonce,
            key_id: stored.key_id,
        };
        Some(
            h.state
                .secrets
                .open(&sealed, aad.as_bytes())
                .unwrap()
                .expose()
                .to_owned(),
        )
    }

    async fn per_user_org(h: &Harness) {
        h.seed("int_org_pu", IntegrationLayer::Org, None, |i| {
            i.credential_mode = CredentialMode::PerUser;
        })
        .await;
    }

    #[tokio::test]
    async fn shared_credential_is_set_by_whoever_manages_the_layer() {
        let expected = [
            (Who::Admin, OK),
            (Who::EngAdmin, OK),
            (Who::Member, FORBIDDEN),
            (Who::Viewer, FORBIDDEN),
            (Who::Other, NOT_FOUND),
            (Who::SalesAdmin, NOT_FOUND),
        ];
        for (who, want) in expected {
            let h = Harness::new().await;
            h.seed("int_space", IntegrationLayer::Space, None, |_| {})
                .await;
            let (status, body) = h.put(who, &uri("int_space"), value("ghp_abc")).await;
            assert_eq!(
                status, want,
                "{who:?} setting a shared space credential: {body}"
            );
        }
    }

    #[tokio::test]
    async fn org_shared_credential_needs_an_org_admin() {
        let h = Harness::new().await;
        h.seed("int_org", IntegrationLayer::Org, None, |_| {}).await;
        let (status, _) = h
            .put(Who::EngAdmin, &uri("int_org"), value("ghp_abc"))
            .await;
        assert_eq!(status, FORBIDDEN);
        let (status, _) = h.put(Who::Admin, &uri("int_org"), value("ghp_abc")).await;
        assert_eq!(status, OK);
    }

    #[tokio::test]
    async fn the_response_reports_presence_and_time_but_never_the_value() {
        let h = Harness::new().await;
        h.seed("int_space", IntegrationLayer::Space, None, |_| {})
            .await;
        let (status, body) = h
            .put(Who::EngAdmin, &uri("int_space"), value("ghp_abc"))
            .await;
        assert_eq!(status, OK);
        assert_eq!(body["has_credential"], json!(true));
        assert!(body["updated_at"].is_string(), "{body}");
        assert!(!body.to_string().contains("ghp_abc"), "{body}");
    }

    #[tokio::test]
    async fn the_value_is_sealed_under_the_integrations_shared_subject() {
        let h = Harness::new().await;
        h.seed("int_space", IntegrationLayer::Space, None, |_| {})
            .await;
        h.put(Who::EngAdmin, &uri("int_space"), value("ghp_abc"))
            .await;

        let stored = h
            .state
            .credentials
            .get("int_space", None)
            .await
            .unwrap()
            .expect("a shared credential row");
        assert!(
            !stored
                .ciphertext
                .windows(b"ghp_abc".len())
                .any(|w| w == b"ghp_abc"),
            "the row must not hold the plain value"
        );
        assert_eq!(
            stored_plaintext(&h, "int_space", None).await.as_deref(),
            Some("ghp_abc")
        );
    }

    #[tokio::test]
    async fn per_user_credentials_belong_to_the_caller() {
        let h = Harness::new().await;
        per_user_org(&h).await;

        let (status, _) = h.put(Who::Member, &uri("int_org_pu"), value("mine")).await;
        assert_eq!(status, OK);
        let (status, _) = h.put(Who::Other, &uri("int_org_pu"), value("theirs")).await;
        assert_eq!(status, OK);

        assert_eq!(
            stored_plaintext(&h, "int_org_pu", Some(Who::Member))
                .await
                .as_deref(),
            Some("mine")
        );
        assert_eq!(
            stored_plaintext(&h, "int_org_pu", Some(Who::Other))
                .await
                .as_deref(),
            Some("theirs")
        );
        assert!(
            !h.has_credential("int_org_pu", None).await,
            "a per-user integration has no shared credential"
        );
    }

    #[tokio::test]
    async fn a_viewer_may_connect_their_own_account() {
        let h = Harness::new().await;
        per_user_org(&h).await;
        let (status, _) = h.put(Who::Viewer, &uri("int_org_pu"), value("mine")).await;
        assert_eq!(status, OK);
    }

    #[tokio::test]
    async fn only_the_owner_sets_the_credential_of_a_personal_integration() {
        let expected = [
            (Who::Member, OK),
            (Who::Admin, FORBIDDEN),
            (Who::Other, NOT_FOUND),
        ];
        for (who, want) in expected {
            let h = Harness::new().await;
            h.seed(
                "int_mine",
                IntegrationLayer::User,
                Some(Who::Member),
                |_| {},
            )
            .await;
            let (status, body) = h.put(who, &uri("int_mine"), value("t")).await;
            assert_eq!(status, want, "{who:?}: {body}");
        }
    }

    #[tokio::test]
    async fn setting_a_credential_makes_the_integration_ready() {
        let h = Harness::new().await;
        per_user_org(&h).await;
        let (_, before) = h.get(Who::Member, "/api/integrations/int_org_pu").await;
        assert_eq!(before["status"], json!("needs_connection"));

        h.put(Who::Member, &uri("int_org_pu"), value("mine")).await;

        let (_, after) = h.get(Who::Member, "/api/integrations/int_org_pu").await;
        assert_eq!(after["status"], json!("ready"), "{after}");
        assert_eq!(after["has_credential"], json!(true));
        let (_, other) = h.get(Who::Other, "/api/integrations/int_org_pu").await;
        assert_eq!(other["status"], json!("needs_connection"));
    }

    #[tokio::test]
    async fn setting_again_replaces_the_value() {
        let h = Harness::new().await;
        h.seed("int_space", IntegrationLayer::Space, None, |_| {})
            .await;
        h.put(Who::EngAdmin, &uri("int_space"), value("first"))
            .await;
        h.put(Who::EngAdmin, &uri("int_space"), value("second"))
            .await;
        assert_eq!(
            stored_plaintext(&h, "int_space", None).await.as_deref(),
            Some("second")
        );
    }

    #[tokio::test]
    async fn clearing_removes_the_callers_credential_only() {
        let h = Harness::new().await;
        per_user_org(&h).await;
        h.put(Who::Member, &uri("int_org_pu"), value("mine")).await;
        h.put(Who::Other, &uri("int_org_pu"), value("theirs")).await;

        let (status, body) = h.delete(Who::Member, &uri("int_org_pu")).await;
        assert_eq!(status, OK);
        assert_eq!(body["has_credential"], json!(false));
        assert!(!h.has_credential("int_org_pu", Some(Who::Member)).await);
        assert!(h.has_credential("int_org_pu", Some(Who::Other)).await);
    }

    #[tokio::test]
    async fn clearing_a_shared_credential_needs_the_same_right_as_setting_it() {
        let h = Harness::new().await;
        h.seed("int_space", IntegrationLayer::Space, None, |_| {})
            .await;
        h.put(Who::EngAdmin, &uri("int_space"), value("t")).await;

        let (status, _) = h.delete(Who::Member, &uri("int_space")).await;
        assert_eq!(status, FORBIDDEN);
        assert!(h.has_credential("int_space", None).await);

        let (status, _) = h.delete(Who::EngAdmin, &uri("int_space")).await;
        assert_eq!(status, OK);
        assert!(!h.has_credential("int_space", None).await);
    }

    #[tokio::test]
    async fn unknown_and_unseen_integrations_are_404() {
        let h = Harness::new().await;
        let (status, _) = h.put(Who::Admin, &uri("int_nope"), value("t")).await;
        assert_eq!(status, NOT_FOUND);
    }

    #[tokio::test]
    async fn integrations_without_a_header_credential_are_rejected() {
        for kind in [IntegrationAuthKind::None, IntegrationAuthKind::Oauth] {
            let h = Harness::new().await;
            h.seed("int_space", IntegrationLayer::Space, None, |i| {
                i.auth_kind = kind;
                i.header_name = None;
                i.header_value_prefix = None;
            })
            .await;
            let (status, body) = h.put(Who::EngAdmin, &uri("int_space"), value("t")).await;
            assert_eq!(status, UNPROCESSABLE, "{kind:?}: {body}");
        }
    }

    #[tokio::test]
    async fn bad_values_are_rejected_without_echoing_them() {
        for bad in ["", "   ", "line\nbreak", "tab\there-is-fine\u{7f}"] {
            let h = Harness::new().await;
            h.seed("int_space", IntegrationLayer::Space, None, |_| {})
                .await;
            let (status, body) = h.put(Who::EngAdmin, &uri("int_space"), value(bad)).await;
            assert_eq!(status, UNPROCESSABLE, "{bad:?}: {body}");
            assert!(!h.has_credential("int_space", None).await);
            if !bad.trim().is_empty() {
                assert!(!body.to_string().contains(bad.trim()), "{body}");
            }
        }
    }

    #[tokio::test]
    async fn surrounding_whitespace_of_a_pasted_value_is_trimmed() {
        let h = Harness::new().await;
        h.seed("int_space", IntegrationLayer::Space, None, |_| {})
            .await;
        h.put(Who::EngAdmin, &uri("int_space"), value("  ghp_abc\n"))
            .await;
        assert_eq!(
            stored_plaintext(&h, "int_space", None).await.as_deref(),
            Some("ghp_abc")
        );
    }

    #[tokio::test]
    async fn a_fixed_header_name_cannot_be_overridden() {
        let h = Harness::new().await;
        h.seed("int_space", IntegrationLayer::Space, None, |_| {})
            .await;
        let (status, _) = h
            .put(
                Who::EngAdmin,
                &uri("int_space"),
                json!({"value": "t", "header_name": "X-Api-Key"}),
            )
            .await;
        assert_eq!(status, UNPROCESSABLE);
    }

    #[tokio::test]
    async fn an_integration_without_a_header_name_takes_the_one_in_the_request() {
        let h = Harness::new().await;
        h.seed("int_space", IntegrationLayer::Space, None, |i| {
            i.catalog_entry_id = None;
            i.header_name = None;
            i.header_value_prefix = None;
        })
        .await;
        let before = h.state.store.get("int_space").await.unwrap().unwrap();

        let (status, body) = h
            .put(
                Who::EngAdmin,
                &uri("int_space"),
                json!({"value": "t", "header_name": "X-Api-Key"}),
            )
            .await;
        assert_eq!(status, OK, "{body}");

        let after = h.state.store.get("int_space").await.unwrap().unwrap();
        assert_eq!(after.header_name.as_deref(), Some("X-Api-Key"));
        assert!(
            after.updated_at > before.updated_at,
            "changing the template must invalidate open connections"
        );
    }

    #[tokio::test]
    async fn a_missing_header_name_is_asked_for() {
        let h = Harness::new().await;
        h.seed("int_space", IntegrationLayer::Space, None, |i| {
            i.catalog_entry_id = None;
            i.header_name = None;
        })
        .await;
        let (status, _) = h.put(Who::EngAdmin, &uri("int_space"), value("t")).await;
        assert_eq!(status, UNPROCESSABLE);
        assert!(!h.has_credential("int_space", None).await);
    }

    #[tokio::test]
    async fn a_member_cannot_name_the_header_of_a_shared_template_they_do_not_manage() {
        let h = Harness::new().await;
        h.seed("int_org_pu", IntegrationLayer::Org, None, |i| {
            i.catalog_entry_id = None;
            i.credential_mode = CredentialMode::PerUser;
            i.header_name = None;
        })
        .await;
        let (status, _) = h
            .put(
                Who::Member,
                &uri("int_org_pu"),
                json!({"value": "t", "header_name": "X-Api-Key"}),
            )
            .await;
        assert_eq!(status, FORBIDDEN);
        assert!(!h.has_credential("int_org_pu", Some(Who::Member)).await);
    }
}
