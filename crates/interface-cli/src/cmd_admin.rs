//! Local administration commands that act on `org.db` directly.
//!
//! These run as the OS user with access to `~/.assistant`, so they work when
//! nobody can sign in: a lost admin password, or an install whose generated
//! credentials file was deleted.

use anyhow::{Context, Result, bail};

use assistant_auth::password::hash_password;
use assistant_auth::session::revoke_sessions;
use assistant_core::auth::RefreshTokenStore;
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::store::{OrgStore, UserStore};
use assistant_storage::OrgPoolFactory;

/// `assistant admin reset-password <email> [--org <slug>]`
pub async fn cmd_admin_reset_password(email: &str, org_slug: &str) -> Result<()> {
    let org_storage = OrgPoolFactory::default_base()?
        .org_storage()
        .await
        .context("failed to open org.db")?;

    let new =
        rpassword::prompt_password("New password:     ").context("failed to read new password")?;
    let confirm = rpassword::prompt_password("Confirm new:      ")
        .context("failed to read password confirmation")?;
    if new != confirm {
        bail!("new password and confirmation do not match");
    }

    reset_password(
        &org_storage.org_store(),
        &org_storage.user_store(),
        &org_storage.refresh_token_store(),
        org_slug,
        email,
        &new,
        &SystemClock,
    )
    .await?;
    println!("Password for {email} reset. Existing sessions were signed out.");
    Ok(())
}

/// Set a new password for `email` in the org `org_slug` and sign the user out
/// everywhere.
async fn reset_password(
    orgs: &dyn OrgStore,
    users: &dyn UserStore,
    refresh_tokens: &dyn RefreshTokenStore,
    org_slug: &str,
    email: &str,
    new_password: &str,
    clock: &dyn Clock,
) -> Result<()> {
    if new_password.is_empty() {
        bail!("new password must not be empty");
    }
    let org = orgs
        .get_org_by_slug(org_slug)
        .await?
        .with_context(|| format!("no organization with slug `{org_slug}`"))?;
    let mut user = users
        .get_user_by_email(&org.id, email)
        .await?
        .with_context(|| format!("no user `{email}` in organization `{org_slug}`"))?;
    if user.idp_issuer.is_some() {
        bail!(
            "`{email}` is managed by an external identity provider — \
             change the password there, not via assistant"
        );
    }

    user.password_hash = hash_password(new_password).context("hashing the new password")?;
    user.updated_at = clock.now();
    users
        .update_user(&user)
        .await
        .context("updating the user")?;
    revoke_sessions(users, refresh_tokens, &user.id)
        .await
        .context("signing the user out")
}

#[cfg(test)]
mod tests {
    use assistant_auth::oauth2::server::InMemoryRefreshTokenStore;
    use assistant_auth::password::verify_password;
    use assistant_core::identity::{OrgId, UserId};
    use assistant_core::store::{InMemoryOrgStore, InMemoryUserStore, Organization, User};

    use super::*;

    struct Fixture {
        orgs: InMemoryOrgStore,
        users: InMemoryUserStore,
        refresh_tokens: InMemoryRefreshTokenStore,
    }

    async fn fixture() -> Fixture {
        let now = chrono::Utc::now();
        let orgs = InMemoryOrgStore::default();
        orgs.create_org(&Organization {
            id: OrgId::from("org_1"),
            name: "Default".into(),
            slug: "default".into(),
            auth_mode: "password".into(),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();
        let users = InMemoryUserStore::default();
        users
            .create_user(&User {
                id: UserId::from("usr_1"),
                org_id: OrgId::from("org_1"),
                email: "admin@localhost".into(),
                name: "Admin".into(),
                password_hash: hash_password("old-password").unwrap(),
                idp_issuer: None,
                idp_subject: None,
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        Fixture {
            orgs,
            users,
            refresh_tokens: InMemoryRefreshTokenStore::default(),
        }
    }

    async fn reset(f: &Fixture, org: &str, email: &str, password: &str) -> Result<()> {
        reset_password(
            &f.orgs,
            &f.users,
            &f.refresh_tokens,
            org,
            email,
            password,
            &SystemClock,
        )
        .await
    }

    #[tokio::test]
    async fn reset_replaces_the_password_and_ends_sessions() {
        let f = fixture().await;
        let id = UserId::from("usr_1");
        let version_before = f
            .users
            .session_state(&id)
            .await
            .unwrap()
            .unwrap()
            .token_version;

        reset(&f, "default", "admin@localhost", "new-password")
            .await
            .unwrap();

        let user = f.users.get_user(&id).await.unwrap().unwrap();
        assert!(verify_password("new-password", &user.password_hash).unwrap());
        assert!(!verify_password("old-password", &user.password_hash).unwrap());
        let version_after = f
            .users
            .session_state(&id)
            .await
            .unwrap()
            .unwrap()
            .token_version;
        assert!(version_after > version_before, "access tokens are revoked");
    }

    #[tokio::test]
    async fn reset_fails_clearly_for_unknown_user_or_org() {
        let f = fixture().await;

        let no_user = reset(&f, "default", "nobody@localhost", "pw").await;
        assert!(format!("{:#}", no_user.unwrap_err()).contains("nobody@localhost"));

        let no_org = reset(&f, "missing", "admin@localhost", "pw").await;
        assert!(format!("{:#}", no_org.unwrap_err()).contains("missing"));
    }

    #[tokio::test]
    async fn reset_rejects_an_empty_password() {
        let f = fixture().await;

        assert!(reset(&f, "default", "admin@localhost", "").await.is_err());

        let user = f
            .users
            .get_user(&UserId::from("usr_1"))
            .await
            .unwrap()
            .unwrap();
        assert!(verify_password("old-password", &user.password_hash).unwrap());
    }

    #[tokio::test]
    async fn reset_refuses_idp_managed_users() {
        let f = fixture().await;
        let id = UserId::from("usr_1");
        let mut user = f.users.get_user(&id).await.unwrap().unwrap();
        user.idp_issuer = Some("https://idp.example".into());
        f.users.update_user(&user).await.unwrap();

        let err = reset(&f, "default", "admin@localhost", "pw")
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("identity provider"));
    }
}
