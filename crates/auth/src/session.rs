//! Ending a user's sessions.

use anyhow::{Context, Result};

use assistant_core::auth::RefreshTokenStore;
use assistant_core::identity::UserId;
use assistant_core::store::UserStore;

/// Sign `user_id` out everywhere.
///
/// Bumping the user's token version makes every access JWT issued so far
/// invalid on its next use; revoking the refresh tokens stops new ones being
/// minted. The version goes first so a failure part-way still ends the access
/// tokens.
pub async fn revoke_sessions(
    users: &dyn UserStore,
    refresh_tokens: &dyn RefreshTokenStore,
    user_id: &UserId,
) -> Result<()> {
    users
        .bump_token_version(user_id)
        .await
        .context("bumping token version")?;
    refresh_tokens
        .revoke_for_user_except(&user_id.0, "")
        .await
        .context("revoking refresh tokens")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use assistant_core::auth::StoredRefreshToken;
    use assistant_core::identity::OrgId;
    use assistant_core::store::{InMemoryUserStore, User};

    use crate::oauth2::server::InMemoryRefreshTokenStore;

    use super::*;

    async fn user_store_with(ids: &[&str]) -> InMemoryUserStore {
        let users = InMemoryUserStore::default();
        let now = Utc::now();
        for id in ids {
            users
                .create_user(&User {
                    id: UserId::from(*id),
                    org_id: OrgId::from("acme"),
                    email: format!("{id}@acme.com"),
                    name: (*id).into(),
                    password_hash: String::new(),
                    idp_issuer: None,
                    idp_subject: None,
                    created_at: now,
                    updated_at: now,
                })
                .await
                .unwrap();
        }
        users
    }

    async fn store_refresh(store: &InMemoryRefreshTokenStore, token: &str, user: &str) {
        store
            .store_token(StoredRefreshToken {
                token: token.into(),
                user_id: user.into(),
                client_id: "webui".into(),
                scopes: vec![],
                resource: None,
                expires_at: Utc::now() + Duration::days(1),
                consumed: false,
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn revokes_access_tokens_and_refresh_tokens_of_that_user_only() {
        let users = user_store_with(&["alice", "bob"]).await;
        let refresh = InMemoryRefreshTokenStore::default();
        store_refresh(&refresh, "alice-web", "alice").await;
        store_refresh(&refresh, "alice-cli", "alice").await;
        store_refresh(&refresh, "bob-web", "bob").await;

        revoke_sessions(&users, &refresh, &UserId::from("alice"))
            .await
            .unwrap();

        let version = |id: &str| {
            let users = &users;
            let id = UserId::from(id);
            async move {
                users
                    .session_state(&id)
                    .await
                    .unwrap()
                    .unwrap()
                    .token_version
            }
        };
        assert_eq!(version("alice").await, 1, "alice's access tokens are void");
        assert_eq!(version("bob").await, 0, "bob is untouched");
        assert!(refresh.get_token("alice-web").await.unwrap().is_none());
        assert!(refresh.get_token("alice-cli").await.unwrap().is_none());
        assert!(refresh.get_token("bob-web").await.unwrap().is_some());
    }
}
