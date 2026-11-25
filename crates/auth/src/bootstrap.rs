//! Bootstrap flows that create initial auth-domain state during installation.
//!
//! Lives in `assistant-auth` because seeding a user with a hashed password is
//! an auth-domain concern; the storage crate only handles the persistence
//! mechanics.

use std::fmt;
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use rand::RngExt;

use assistant_core::clock::Clock;
use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
use assistant_core::store::{MembershipStore, SpaceMembership, User, UserStore};

use crate::password::hash_password;

/// Email address of the initial admin user.
pub const ADMIN_EMAIL: &str = "admin@localhost";

/// Length of the generated initial admin password.
const ADMIN_PASSWORD_LEN: usize = 24;

/// The initial admin user and the password generated for them.
///
/// `Debug` redacts the password so it cannot leak through logs.
pub struct AdminCredentials {
    pub user_id: UserId,
    pub email: String,
    pub password: String,
}

impl fmt::Debug for AdminCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdminCredentials")
            .field("user_id", &self.user_id)
            .field("email", &self.email)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// Create the initial admin user of `org_id` unless the org already has users.
///
/// The admin gets a random password and the `OrgAdmin` role in `space_id`. The
/// password is written to `credentials_file` (mode 0600 on Unix) so the
/// operator can sign in; it is never logged. Returns `None`, touching nothing,
/// when the org already has at least one user.
///
/// If the file cannot be written the user is removed again and the error is
/// returned, so the next start retries instead of leaving an admin nobody can
/// sign in as.
pub async fn ensure_admin_user(
    user_store: &dyn UserStore,
    membership_store: &dyn MembershipStore,
    org_id: &OrgId,
    space_id: &SpaceId,
    credentials_file: &Path,
    clock: &dyn Clock,
) -> Result<Option<AdminCredentials>> {
    if !user_store
        .list_users(org_id)
        .await
        .context("listing users")?
        .is_empty()
    {
        return Ok(None);
    }

    let credentials =
        create_admin_user_with_clock(user_store, membership_store, org_id, space_id, clock).await?;

    if let Err(e) = write_credentials_file(credentials_file, &credentials) {
        if let Err(cleanup) = membership_store
            .remove_membership(&credentials.user_id, space_id)
            .await
        {
            tracing::warn!("rolling back admin membership failed: {cleanup:#}");
        }
        if let Err(cleanup) = user_store.delete_user(&credentials.user_id).await {
            tracing::warn!("rolling back admin user failed: {cleanup:#}");
        }
        return Err(e.context(format!(
            "writing admin credentials to {}",
            credentials_file.display()
        )));
    }

    Ok(Some(credentials))
}

/// Create the initial admin user with a random password.
///
/// Grants the new user the `OrgAdmin` role in `space_id`. Tests pass a
/// `FakeClock` to assert `created_at` / `updated_at` against virtual time.
pub async fn create_admin_user_with_clock(
    user_store: &dyn UserStore,
    membership_store: &dyn MembershipStore,
    org_id: &OrgId,
    space_id: &SpaceId,
    clock: &dyn Clock,
) -> Result<AdminCredentials> {
    let now = clock.now();

    let password: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(ADMIN_PASSWORD_LEN)
        .map(char::from)
        .collect();
    let password_hash = hash_password(&password).context("hashing admin password")?;

    let user_id = UserId::from(format!("usr_{}", uuid::Uuid::new_v4()));
    let user = User {
        id: user_id.clone(),
        org_id: org_id.clone(),
        email: ADMIN_EMAIL.into(),
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
        email: ADMIN_EMAIL.into(),
        password,
    })
}

/// Write `credentials` to `path`, readable by the owner only.
fn write_credentials_file(path: &Path, credentials: &AdminCredentials) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating credentials directory")?;
    }

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path).context("opening credentials file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        // `mode` only applies to files created here; tighten a leftover file.
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .context("restricting credentials file permissions")?;
    }

    writeln!(
        file,
        "# Initial admin credentials. Sign in, change the password, then delete this file.\n\
         email: {}\n\
         password: {}",
        credentials.email, credentials.password
    )
    .context("writing credentials file")
}

#[cfg(test)]
mod tests {
    use assistant_core::clock::SystemClock;
    use assistant_core::store::{InMemoryMembershipStore, InMemoryUserStore};

    use crate::password::verify_password;

    use super::*;

    fn org() -> OrgId {
        OrgId::from("org_1")
    }

    fn space() -> SpaceId {
        SpaceId::from("spc_1")
    }

    async fn ensure(
        users: &InMemoryUserStore,
        memberships: &InMemoryMembershipStore,
        file: &Path,
    ) -> Result<Option<AdminCredentials>> {
        ensure_admin_user(users, memberships, &org(), &space(), file, &SystemClock).await
    }

    #[tokio::test]
    async fn first_start_creates_one_admin_and_the_credentials_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("orgs/default/admin_credentials.txt");
        let (users, memberships) = (
            InMemoryUserStore::default(),
            InMemoryMembershipStore::default(),
        );

        let creds = ensure(&users, &memberships, &file)
            .await
            .unwrap()
            .expect("an empty org gets an admin");

        let stored = users.list_users(&org()).await.unwrap();
        assert_eq!(stored.len(), 1, "exactly one admin");
        assert_eq!(stored[0].email, ADMIN_EMAIL);
        assert!(
            verify_password(&creds.password, &stored[0].password_hash).unwrap(),
            "the returned password matches the stored hash"
        );
        let roles = memberships.get_space_roles(&stored[0].id).await.unwrap();
        assert_eq!(roles.get(&space()), Some(&Role::OrgAdmin));

        let written = std::fs::read_to_string(&file).unwrap();
        assert!(
            written.contains(&format!("email: {ADMIN_EMAIL}")),
            "{written}"
        );
        assert!(
            written.contains(&format!("password: {}", creds.password)),
            "the file carries the password"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn credentials_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("admin_credentials.txt");
        // A leftover file with loose permissions must be tightened too.
        std::fs::write(&file, "stale").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();

        ensure(
            &InMemoryUserStore::default(),
            &InMemoryMembershipStore::default(),
            &file,
        )
        .await
        .unwrap();

        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[tokio::test]
    async fn second_start_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("admin_credentials.txt");
        let (users, memberships) = (
            InMemoryUserStore::default(),
            InMemoryMembershipStore::default(),
        );
        ensure(&users, &memberships, &file).await.unwrap();
        let before = std::fs::read_to_string(&file).unwrap();

        let again = ensure(&users, &memberships, &file).await.unwrap();

        assert!(again.is_none(), "an org with users is left alone");
        assert_eq!(users.list_users(&org()).await.unwrap().len(), 1);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    }

    #[tokio::test]
    async fn an_org_that_already_has_a_user_gets_no_admin() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("admin_credentials.txt");
        let (users, memberships) = (
            InMemoryUserStore::default(),
            InMemoryMembershipStore::default(),
        );
        let now = chrono::Utc::now();
        users
            .create_user(&User {
                id: UserId::from("usr_existing"),
                org_id: org(),
                email: "someone@example.com".into(),
                name: "Someone".into(),
                password_hash: String::new(),
                idp_issuer: None,
                idp_subject: None,
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();

        let created = ensure(&users, &memberships, &file).await.unwrap();

        assert!(created.is_none());
        assert!(!file.exists(), "no credentials file for an existing org");
    }

    #[tokio::test]
    async fn unwritable_credentials_file_leaves_no_admin_behind() {
        let dir = tempfile::tempdir().unwrap();
        // The parent of the target is a regular file, so the write must fail.
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        let file = blocker.join("admin_credentials.txt");
        let (users, memberships) = (
            InMemoryUserStore::default(),
            InMemoryMembershipStore::default(),
        );

        let result = ensure(&users, &memberships, &file).await;

        assert!(result.is_err());
        assert!(
            users.list_users(&org()).await.unwrap().is_empty(),
            "the next start must be able to retry"
        );
    }

    #[test]
    fn debug_output_redacts_the_password() {
        let creds = AdminCredentials {
            user_id: UserId::from("usr_1"),
            email: ADMIN_EMAIL.into(),
            password: "hunter2-hunter2".into(),
        };
        assert!(!format!("{creds:?}").contains("hunter2"));
    }
}
