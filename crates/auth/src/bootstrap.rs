//! Bootstrap flows that create initial auth-domain state during installation
//! or legacy migration.
//!
//! Lives in `assistant-auth` because seeding a user with a hashed password is
//! an auth-domain concern; the storage crate only handles the persistence
//! mechanics.

use anyhow::{Context, Result};
use rand::RngExt;
use tracing::info;

use assistant_core::clock::{Clock, SystemClock};
use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
use assistant_core::store::{MembershipStore, SpaceMembership, User, UserStore};

use crate::password::hash_password;

/// Credentials returned by [`create_admin_user`] so the caller can display
/// them through the appropriate output channel (e.g. `info!` log, CLI
/// banner, etc.) instead of writing directly to stdout from library code.
#[derive(Debug)]
pub struct AdminCredentials {
    pub user_id: UserId,
    pub email: String,
    /// `Some(password)` when a random password was generated.
    /// `None` when `ASSISTANT_WEB_TOKEN` was used (caller already knows it).
    pub generated_password: Option<String>,
}

/// Create the initial admin user during legacy migration using the system clock.
///
/// Convenience wrapper around [`create_admin_user_with_clock`].
pub async fn create_admin_user(
    user_store: &dyn UserStore,
    membership_store: &dyn MembershipStore,
    org_id: &OrgId,
    space_id: &SpaceId,
) -> Result<AdminCredentials> {
    create_admin_user_with_clock(user_store, membership_store, org_id, space_id, &SystemClock).await
}

/// Create the initial admin user during legacy migration.
///
/// - If `ASSISTANT_WEB_TOKEN` is set, uses it as a temporary password.
/// - Otherwise, generates a random 24-character password.
///
/// Grants the new user the `OrgAdmin` role in `space_id`. Returns
/// [`AdminCredentials`] so the caller can display them.
///
/// Tests pass a `FakeClock` to assert `created_at` / `updated_at`
/// timestamps against virtual time.
pub async fn create_admin_user_with_clock(
    user_store: &dyn UserStore,
    membership_store: &dyn MembershipStore,
    org_id: &OrgId,
    space_id: &SpaceId,
    clock: &dyn Clock,
) -> Result<AdminCredentials> {
    let now = clock.now();

    // Determine password source.
    let (password, was_generated) = match std::env::var("ASSISTANT_WEB_TOKEN") {
        Ok(token) if !token.is_empty() => {
            info!("using ASSISTANT_WEB_TOKEN as initial admin password");
            (token, false)
        }
        _ => {
            // Generate a random 24-character alphanumeric password.
            let pw: String = rand::rng()
                .sample_iter(&rand::distr::Alphanumeric)
                .take(24)
                .map(char::from)
                .collect();
            (pw, true)
        }
    };

    let password_hash = hash_password(&password).context("hashing admin password")?;

    let user_id = UserId::from(format!("usr_{}", uuid::Uuid::new_v4()));
    let user = User {
        id: user_id.clone(),
        org_id: org_id.clone(),
        email: "admin@localhost".into(),
        name: "Admin".into(),
        password_hash,
        idp_issuer: None,
        idp_subject: None,
        created_at: now,
        updated_at: now,
    };

    user_store
        .create_user(&user)
        .await
        .context("creating admin user")?;

    // Grant OrgAdmin role in default space.
    let membership = SpaceMembership {
        user_id: user_id.clone(),
        space_id: space_id.clone(),
        role: Role::OrgAdmin,
        created_at: now,
    };
    membership_store
        .add_membership(&membership)
        .await
        .context("granting admin membership")?;

    Ok(AdminCredentials {
        user_id,
        email: "admin@localhost".into(),
        generated_password: if was_generated { Some(password) } else { None },
    })
}
