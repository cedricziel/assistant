//! First-run install / legacy-layout migration helper shared by the web-ui and
//! orchestrator binaries.
//!
//! This module composes the four steps that turn a legacy single-user install
//! (`~/.assistant/assistant.db` + flat `agents/` + `skills/`) into the multi-org
//! layout (`orgs/default/spaces/default/space.db` + seeded `org.db` + admin
//! user). The composition lives here — and not in `assistant-storage` — because
//! `assistant-storage` deliberately keeps `assistant-auth` and `assistant-backup`
//! as `dev-dependencies` only, so production storage code stays
//! domain-decoupled.
//!
//! Both binaries depend on `assistant-web-ui` (the CLI uses it as a library to
//! launch the embedded web server), so this is the lowest common dependency
//! point that already has visibility into all three crates.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::info;

use assistant_auth::bootstrap::{AdminCredentials, create_admin_user};
use assistant_storage::migration;

/// Outcome of a successful legacy-to-multi-org migration. `None` is returned by
/// [`ensure_migrated`] when there is nothing to do.
#[derive(Debug)]
pub struct MigrationOutcome {
    /// Path of the pre-migration backup archive.
    pub backup_path: PathBuf,
    /// Initial admin credentials produced by `assistant_auth::bootstrap`.
    pub admin: AdminCredentials,
    /// Path of `admin-credentials.txt` when a random password was generated.
    /// `None` when `ASSISTANT_WEB_TOKEN` was used as the seed password.
    pub admin_credentials_file: Option<PathBuf>,
}

/// Run the full legacy-to-multi-org migration if and only if `base_path` still
/// has the legacy layout. Idempotent: returns `Ok(None)` when the install has
/// already been migrated.
///
/// Sequences:
///
/// 1. `assistant_backup::backup_legacy_install` — pre-migration tarball.
/// 2. `assistant_storage::migration::migrate_filesystem`.
/// 3. `assistant_storage::migration::migrate_database` (atomic cutover: renames
///    `assistant.db` → `assistant.db.legacy` and drops WAL/SHM sidecars).
/// 4. `assistant_auth::bootstrap::create_admin_user`, plus a 0600
///    `admin-credentials.txt` when the password was generated.
pub async fn ensure_migrated(base_path: &Path) -> Result<Option<MigrationOutcome>> {
    if !migration::is_legacy_layout(base_path) {
        return Ok(None);
    }

    info!("detected legacy single-user layout — running auto-migration");

    let backup_path = assistant_backup::backup_legacy_install(base_path)
        .await
        .context("creating pre-migration backup")?;

    migration::migrate_filesystem(base_path)
        .await
        .context("legacy → multi-org filesystem migration failed")?;

    let (org_storage, org_id, space_id) = migration::migrate_database(base_path)
        .await
        .context("legacy → multi-org database migration failed")?;

    let user_store = org_storage.user_store();
    let membership_store = org_storage.membership_store();
    let admin = create_admin_user(&user_store, &membership_store, &org_id, &space_id)
        .await
        .context("creating initial admin user")?;
    drop(org_storage);

    let admin_credentials_file = match admin.generated_password.as_ref() {
        Some(pw) => {
            let path = base_path.join("admin-credentials.txt");
            write_admin_credentials_file(&path, &admin.email, pw)
                .with_context(|| format!("writing admin credentials to {}", path.display()))?;
            Some(path)
        }
        None => None,
    };

    Ok(Some(MigrationOutcome {
        backup_path,
        admin,
        admin_credentials_file,
    }))
}

/// Write initial admin credentials to a 0600 file. Refuses to overwrite an
/// existing file or follow symlinks (uses `create_new`).
pub(crate) fn write_admin_credentials_file(path: &Path, email: &str, password: &str) -> Result<()> {
    use std::io::Write as _;

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        opts.mode(0o600);
    }
    let mut f = opts
        .open(path)
        .with_context(|| format!("opening {} for writing", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        f.set_permissions(std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("restricting permissions on {}", path.display()))?;
    }
    writeln!(
        f,
        "# Initial admin credentials for assistant web-ui.\n\
         # Delete this file after recording the password and changing it on first login.\n\
         email={email}\n\
         password={password}"
    )
    .with_context(|| format!("writing credentials to {}", path.display()))?;
    f.sync_all()
        .with_context(|| format!("fsync of {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn ensure_migrated_runs_full_pipeline_on_legacy_layout() {
        let dir = TempDir::new().unwrap();
        let legacy_db = dir.path().join("assistant.db");

        // Seed a real legacy layout with a populated DB so migrate_database
        // has something to copy.
        let storage = assistant_storage::StorageLayer::new(&legacy_db)
            .await
            .unwrap();
        drop(storage);

        // SAFETY: scoped to this test process.
        unsafe {
            std::env::remove_var("ASSISTANT_WEB_TOKEN");
        }

        let outcome = ensure_migrated(dir.path())
            .await
            .expect("ensure_migrated should succeed on legacy layout")
            .expect("legacy layout should produce a migration outcome");

        assert!(
            outcome.backup_path.exists(),
            "backup archive should be created"
        );
        assert!(
            dir.path()
                .join("orgs/default/spaces/default/space.db")
                .exists(),
            "space.db should exist after migration"
        );
        assert!(
            dir.path().join("assistant.db.legacy").exists(),
            "assistant.db should be renamed to .legacy"
        );
        assert!(
            !legacy_db.exists(),
            "assistant.db should not exist after cutover"
        );
        let creds_file = outcome
            .admin_credentials_file
            .expect("generated password should produce a credentials file");
        assert!(creds_file.exists(), "credentials file should be written");
    }

    #[tokio::test]
    async fn ensure_migrated_is_idempotent_on_migrated_layout() {
        let dir = TempDir::new().unwrap();

        // Already-migrated state: orgs/ exists and no assistant.db. The helper
        // must short-circuit without touching anything.
        std::fs::create_dir_all(dir.path().join("orgs/default/spaces/default")).unwrap();

        let result = ensure_migrated(dir.path())
            .await
            .expect("ensure_migrated should not fail on already-migrated layout");

        assert!(
            result.is_none(),
            "ensure_migrated must return None when there is no legacy layout"
        );

        assert!(
            !dir.path().join("assistant.db.legacy").exists(),
            "no rename should happen on already-migrated layout"
        );
        assert!(
            !dir.path().join("admin-credentials.txt").exists(),
            "no admin-credentials.txt should be written on no-op path"
        );
    }
}
