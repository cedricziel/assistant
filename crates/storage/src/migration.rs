//! Legacy-to-multi-org migration for existing single-user installations.
//!
//! Detects the legacy `~/.assistant/` layout (flat `assistant.db` + `agents/` +
//! `skills/` + `config.toml`) and migrates the filesystem and databases to the
//! new org/space directory structure introduced by the multi-user feature.
//!
//! Storage owns the schema-shaped migration steps:
//!
//! 1. **Detect** — [`is_legacy_layout`] checks for `assistant.db` without `orgs/`.
//! 2. **Filesystem** — [`migrate_filesystem`] creates `orgs/default/spaces/default/`
//!    and copies `agents/`, `skills/`, interface configs.
//! 3. **Database** — [`migrate_database`] copies `assistant.db` → `space.db` and
//!    seeds `org.db` with the default org and space.
//!
//! The pre-migration backup (provided by `assistant-backup::backup_legacy_install`)
//! and admin-user bootstrap (provided by `assistant-auth::bootstrap::create_admin_user`)
//! are sequenced by the caller so this crate keeps a clean dependency boundary.

use std::path::Path;

use anyhow::{Context, Result};
use tracing::{info, warn};

use assistant_core::identity::{OrgId, SpaceId};

use crate::org_storage::OrgStorageLayer;

// -- Constants ---------------------------------------------------------------

/// Default org slug used when migrating a legacy single-user installation.
pub const DEFAULT_ORG_SLUG: &str = "default";
/// Default space slug used when migrating a legacy single-user installation.
pub const DEFAULT_SPACE_SLUG: &str = "default";
/// Default org name.
const DEFAULT_ORG_NAME: &str = "Default Organization";
/// Default space name.
const DEFAULT_SPACE_NAME: &str = "Default Space";

// -- Detection ---------------------------------------------------------------

/// Check whether `base_path` contains a legacy (pre-multi-org) layout.
///
/// Returns `true` when `assistant.db` exists **and** there is no `orgs/`
/// directory — i.e. the installation has not yet been migrated.
pub fn is_legacy_layout(base_path: &Path) -> bool {
    let has_db = base_path.join("assistant.db").exists();
    let has_orgs = base_path.join("orgs").is_dir();
    has_db && !has_orgs
}

// -- Filesystem migration ----------------------------------------------------

/// Create the new org/space directory structure and copy legacy files into it.
///
/// Creates:
/// ```text
/// base_path/orgs/default/spaces/default/agents/
/// base_path/orgs/default/spaces/default/skills/
/// ```
///
/// Copies `agents/` and `skills/` directory trees. Splits `config.toml` into
/// `server.toml` (global settings) and `orgs/default/org.toml` (org settings).
pub async fn migrate_filesystem(base_path: &Path) -> Result<()> {
    let space_dir = base_path
        .join("orgs")
        .join(DEFAULT_ORG_SLUG)
        .join("spaces")
        .join(DEFAULT_SPACE_SLUG);
    let org_dir = base_path.join("orgs").join(DEFAULT_ORG_SLUG);

    // Create the directory structure.
    tokio::fs::create_dir_all(&space_dir)
        .await
        .with_context(|| format!("creating space directory: {}", space_dir.display()))?;

    // Copy agents/ → orgs/default/spaces/default/agents/
    let legacy_agents = base_path.join("agents");
    if legacy_agents.is_dir() {
        let dest = space_dir.join("agents");
        copy_dir_recursive(&legacy_agents, &dest)
            .await
            .context("copying agents directory")?;
        info!("copied agents/ → {}", dest.display());
    }

    // Copy skills/ → orgs/default/spaces/default/skills/
    let legacy_skills = base_path.join("skills");
    if legacy_skills.is_dir() {
        let dest = space_dir.join("skills");
        copy_dir_recursive(&legacy_skills, &dest)
            .await
            .context("copying skills directory")?;
        info!("copied skills/ → {}", dest.display());
    }

    // Split config.toml → server.toml + org.toml
    let legacy_config = base_path.join("config.toml");
    if legacy_config.is_file() {
        let content = tokio::fs::read_to_string(&legacy_config)
            .await
            .context("reading legacy config.toml")?;

        let (server_toml, org_toml) = split_config(&content);

        let server_path = base_path.join("server.toml");
        tokio::fs::write(&server_path, server_toml.as_bytes())
            .await
            .with_context(|| format!("writing {}", server_path.display()))?;
        info!("wrote server.toml");

        let org_toml_path = org_dir.join("org.toml");
        tokio::fs::write(&org_toml_path, org_toml.as_bytes())
            .await
            .with_context(|| format!("writing {}", org_toml_path.display()))?;
        info!("wrote org.toml");
    }

    Ok(())
}

/// Split legacy `config.toml` into server-level and org-level config.
///
/// Server-level sections: `[storage]`, `[bus]`, `[mcp]`, `[self_improvement]`
/// Org-level sections: `[llm]`, `[agent]`, `[skills]`, `[memory]`,
///   `[transcription]`, `[tts]`, `[learning]`, `[signal]`, `[slack]`,
///   `[matrix]`, `[mattermost]`, `[nextcloud]`
fn split_config(content: &str) -> (String, String) {
    let server_sections = ["[storage]", "[bus]", "[mcp]", "[self_improvement]"];

    let mut server_lines = Vec::new();
    let mut org_lines = Vec::new();
    let mut in_server_section = false;
    let mut header_done = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect section headers.
        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            let is_server = server_sections.iter().any(|s| trimmed.starts_with(s));

            in_server_section = is_server;
            header_done = true;

            if is_server {
                server_lines.push(line.to_string());
            } else {
                org_lines.push(line.to_string());
            }
            continue;
        }

        // Subsections like [[mcp.servers]] stay with their parent.
        if trimmed.starts_with("[[") {
            if trimmed.contains("mcp.") {
                in_server_section = true;
                server_lines.push(line.to_string());
            } else {
                in_server_section = false;
                org_lines.push(line.to_string());
            }
            continue;
        }

        if !header_done {
            // Lines before the first section header: comments and blanks go
            // to both files; non-comment scalar keys go to server only
            // (legacy top-level keys like `db_path` map to storage config).
            let is_comment_or_blank =
                trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';');
            server_lines.push(line.to_string());
            if is_comment_or_blank {
                org_lines.push(line.to_string());
            }
        } else if in_server_section {
            server_lines.push(line.to_string());
        } else {
            org_lines.push(line.to_string());
        }
    }

    let server = server_lines.join("\n") + "\n";
    let org = org_lines.join("\n") + "\n";
    (server, org)
}

/// Recursively copy a directory tree.
async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    tokio::fs::create_dir_all(dst)
        .await
        .with_context(|| format!("creating directory: {}", dst.display()))?;

    let mut rd = tokio::fs::read_dir(src)
        .await
        .with_context(|| format!("reading directory: {}", src.display()))?;

    while let Some(entry) = rd.next_entry().await? {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        let meta = entry.metadata().await?;
        if meta.is_dir() {
            Box::pin(copy_dir_recursive(&src_path, &dst_path)).await?;
        } else if meta.is_file() {
            tokio::fs::copy(&src_path, &dst_path)
                .await
                .with_context(|| {
                    format!("copying {} → {}", src_path.display(), dst_path.display())
                })?;
        }
    }

    Ok(())
}

// -- Database migration ------------------------------------------------------

/// Copy `assistant.db` to `space.db` and create `org.db` seeded with the
/// default org and space.
///
/// - `assistant.db` → `orgs/default/spaces/default/space.db` (verbatim copy;
///   existing space-level migrations will apply on next startup).
/// - Creates `org.db` (at the installation root) with org schema populated with:
///   - Default organization row
///   - Default space row
///
/// Returns the opened [`OrgStorageLayer`] together with the seeded
/// [`OrgId`]/[`SpaceId`] so the caller can layer auth-domain bootstrap
/// (admin user creation) on top without forcing storage to depend on
/// `assistant-auth`.
pub async fn migrate_database(base_path: &Path) -> Result<(OrgStorageLayer, OrgId, SpaceId)> {
    let legacy_db = base_path.join("assistant.db");
    let org_dir = base_path.join("orgs").join(DEFAULT_ORG_SLUG);
    let space_dir = org_dir.join("spaces").join(DEFAULT_SPACE_SLUG);

    tokio::fs::create_dir_all(&space_dir)
        .await
        .with_context(|| format!("creating space directory: {}", space_dir.display()))?;

    // Copy assistant.db → space.db
    let space_db = space_dir.join("space.db");
    tokio::fs::copy(&legacy_db, &space_db)
        .await
        .with_context(|| format!("copying {} → {}", legacy_db.display(), space_db.display()))?;
    info!("copied assistant.db → {}", space_db.display());

    // Also copy WAL/SHM if present.
    // Note: callers should ensure the source database is closed (no active
    // connections) before invoking this migration so that the WAL is
    // checkpointed. If the files still exist, we copy them on a best-effort
    // basis and warn on failure.
    for suffix in &["-wal", "-shm"] {
        let src = base_path.join(format!("assistant.db{suffix}"));
        if src.exists() {
            let dst = space_dir.join(format!("space.db{suffix}"));
            if let Err(e) = tokio::fs::copy(&src, &dst).await {
                warn!(src = %src.display(), dst = %dst.display(), error = %e, "failed to copy WAL/SHM file");
            }
        }
    }

    // Create org.db at the installation root (next to assistant.db), matching
    // the path resolved by `OrgPoolFactory::org_db_path()`.
    let org_db_path = base_path.join("org.db");
    let org_storage = OrgStorageLayer::new(&org_db_path)
        .await
        .context("creating org.db")?;

    // Seed default org.
    use assistant_core::store::{OrgStore, Organization, Space, SpaceStore};
    let now = chrono::Utc::now();

    let org = Organization {
        id: OrgId::from(format!("org_{}", uuid::Uuid::new_v4())),
        name: DEFAULT_ORG_NAME.into(),
        slug: DEFAULT_ORG_SLUG.into(),
        auth_mode: "password".into(),
        created_at: now,
        updated_at: now,
    };
    org_storage
        .org_store()
        .create_org(&org)
        .await
        .context("creating default organization")?;
    info!("created default organization: {}", org.id);

    // Seed default space.
    let space = Space {
        id: SpaceId::from(format!("spc_{}", uuid::Uuid::new_v4())),
        org_id: org.id.clone(),
        name: DEFAULT_SPACE_NAME.into(),
        slug: DEFAULT_SPACE_SLUG.into(),
        created_at: now,
        updated_at: now,
    };
    org_storage
        .space_store()
        .create_space(&space)
        .await
        .context("creating default space")?;
    info!("created default space: {}", space.id);

    // Cut over: rename assistant.db so any process re-resolving the legacy
    // path fails loudly instead of silently diverging from space.db. The
    // sidecar files no longer correspond to a live database after rename;
    // remove them so the legacy SQLite WAL state can't be reused.
    let renamed = base_path.join("assistant.db.legacy");
    tokio::fs::rename(&legacy_db, &renamed)
        .await
        .with_context(|| format!("renaming {} → {}", legacy_db.display(), renamed.display()))?;
    info!("renamed {} → {}", legacy_db.display(), renamed.display());

    for suffix in &["-wal", "-shm"] {
        let sidecar = base_path.join(format!("assistant.db{suffix}"));
        if sidecar.exists()
            && let Err(e) = tokio::fs::remove_file(&sidecar).await
        {
            warn!(
                path = %sidecar.display(),
                error = %e,
                "failed to remove legacy sidecar"
            );
        }
    }

    Ok((org_storage, org.id, space.id))
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    use assistant_core::store::{MembershipStore, OrgStore, SpaceStore, UserStore};

    // -- is_legacy_layout tests ----------------------------------------------

    #[test]
    fn detects_legacy_layout() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("assistant.db"), b"").unwrap();

        assert!(
            is_legacy_layout(dir.path()),
            "directory with assistant.db and no orgs/ should be detected as legacy"
        );
    }

    #[test]
    fn already_migrated_is_not_legacy() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("assistant.db"), b"").unwrap();
        std::fs::create_dir(dir.path().join("orgs")).unwrap();

        assert!(
            !is_legacy_layout(dir.path()),
            "directory with both assistant.db and orgs/ should NOT be detected as legacy"
        );
    }

    #[test]
    fn empty_dir_is_not_legacy() {
        let dir = TempDir::new().unwrap();

        assert!(
            !is_legacy_layout(dir.path()),
            "empty directory should NOT be detected as legacy"
        );
    }

    #[test]
    fn fresh_install_no_db_is_not_legacy() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("orgs")).unwrap();

        assert!(
            !is_legacy_layout(dir.path()),
            "fresh install with orgs/ but no assistant.db should NOT be legacy"
        );
    }

    // -- split_config tests --------------------------------------------------

    #[test]
    fn split_config_separates_sections() {
        let config = r#"# Header comment

[llm]
provider = "ollama"
model = "qwen2.5:7b"

[storage]
# db_path = "~/.assistant/assistant.db"

[agent]
# id = "default"

[mcp]

[[mcp.servers]]
name = "github"

[skills]
# extra_dirs = []

[self_improvement]
trace_enabled = true

[memory]
# enabled = true

[learning]
enabled = true
"#;
        let (server, org) = split_config(config);

        // Server should have storage, mcp, self_improvement
        assert!(
            server.contains("[storage]"),
            "server.toml should contain [storage]"
        );
        assert!(server.contains("[mcp]"), "server.toml should contain [mcp]");
        assert!(
            server.contains("[[mcp.servers]]"),
            "server.toml should contain [[mcp.servers]]"
        );
        assert!(
            server.contains("[self_improvement]"),
            "server.toml should contain [self_improvement]"
        );

        // Server should NOT have llm, agent, skills, memory, learning
        assert!(
            !server.contains("[llm]"),
            "server.toml should NOT contain [llm]"
        );
        assert!(
            !server.contains("[agent]"),
            "server.toml should NOT contain [agent]"
        );
        assert!(
            !server.contains("[learning]"),
            "server.toml should NOT contain [learning]"
        );

        // Org should have llm, agent, skills, memory, learning
        assert!(org.contains("[llm]"), "org.toml should contain [llm]");
        assert!(org.contains("[agent]"), "org.toml should contain [agent]");
        assert!(org.contains("[skills]"), "org.toml should contain [skills]");
        assert!(org.contains("[memory]"), "org.toml should contain [memory]");
        assert!(
            org.contains("[learning]"),
            "org.toml should contain [learning]"
        );

        // Org should NOT have storage, mcp, self_improvement
        assert!(
            !org.contains("[storage]"),
            "org.toml should NOT contain [storage]"
        );
        assert!(!org.contains("[mcp]"), "org.toml should NOT contain [mcp]");
    }

    // -- copy_dir_recursive tests --------------------------------------------

    #[tokio::test]
    async fn copy_dir_recursive_copies_tree() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        // Create a nested structure.
        std::fs::create_dir_all(src.path().join("a/b")).unwrap();
        std::fs::write(src.path().join("a/file1.txt"), b"hello").unwrap();
        std::fs::write(src.path().join("a/b/file2.txt"), b"world").unwrap();

        let dst_path = dst.path().join("copied");
        copy_dir_recursive(src.path().join("a").as_path(), &dst_path)
            .await
            .unwrap();

        assert!(dst_path.join("file1.txt").exists());
        assert!(dst_path.join("b/file2.txt").exists());

        let content = std::fs::read_to_string(dst_path.join("b/file2.txt")).unwrap();
        assert_eq!(content, "world");
    }

    // -- migrate_filesystem tests --------------------------------------------

    #[tokio::test]
    async fn migrate_filesystem_creates_structure() {
        let dir = TempDir::new().unwrap();

        // Seed legacy layout.
        std::fs::create_dir_all(dir.path().join("agents/default")).unwrap();
        std::fs::write(dir.path().join("agents/default/SOUL.md"), b"# Soul").unwrap();
        std::fs::create_dir_all(dir.path().join("skills/web-fetch")).unwrap();
        std::fs::write(dir.path().join("skills/web-fetch/SKILL.md"), b"# Web Fetch").unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "[llm]\nprovider = \"ollama\"\n\n[storage]\n# db_path\n",
        )
        .unwrap();

        migrate_filesystem(dir.path()).await.unwrap();

        let space_dir = dir.path().join("orgs/default/spaces/default");

        // Agents copied.
        assert!(
            space_dir.join("agents/default/SOUL.md").exists(),
            "agents should be copied to new location"
        );

        // Skills copied.
        assert!(
            space_dir.join("skills/web-fetch/SKILL.md").exists(),
            "skills should be copied to new location"
        );

        // server.toml created.
        assert!(
            dir.path().join("server.toml").exists(),
            "server.toml should be created"
        );
        let server = std::fs::read_to_string(dir.path().join("server.toml")).unwrap();
        assert!(
            server.contains("[storage]"),
            "server.toml should contain [storage]"
        );

        // org.toml created.
        let org_toml_path = dir.path().join("orgs/default/org.toml");
        assert!(org_toml_path.exists(), "org.toml should be created");
        let org = std::fs::read_to_string(org_toml_path).unwrap();
        assert!(org.contains("[llm]"), "org.toml should contain [llm]");
    }

    // -- migrate_database tests ----------------------------------------------

    #[tokio::test]
    async fn migrate_database_creates_org_and_space_dbs() {
        let dir = TempDir::new().unwrap();

        // Create a valid legacy database with migrations, then close it so
        // the WAL is checkpointed before migrate_database copies the file.
        let legacy_db = dir.path().join("assistant.db");
        let storage = crate::StorageLayer::new(&legacy_db).await.unwrap();
        drop(storage);

        let (org_storage, org_id, space_id) = migrate_database(dir.path()).await.unwrap();

        let space_db = dir.path().join("orgs/default/spaces/default/space.db");
        assert!(
            space_db.exists(),
            "space.db should be created by copying assistant.db"
        );

        let org_db = dir.path().join("org.db");
        assert!(org_db.exists(), "org.db should be created at install root");

        // Verify org.db has a default organization.
        let orgs = org_storage.org_store().list_orgs().await.unwrap();
        assert_eq!(orgs.len(), 1, "org.db should have exactly one organization");
        assert_eq!(orgs[0].slug, "default");
        assert_eq!(orgs[0].id, org_id);

        // Verify there's a default space.
        let spaces = org_storage
            .space_store()
            .list_spaces(&orgs[0].id)
            .await
            .unwrap();
        assert_eq!(spaces.len(), 1, "org.db should have exactly one space");
        assert_eq!(spaces[0].slug, "default");
        assert_eq!(spaces[0].id, space_id);

        // No users should be created — that is now the caller's responsibility
        // (handled via assistant_auth::bootstrap::create_admin_user).
        let users = org_storage
            .user_store()
            .list_users(&orgs[0].id)
            .await
            .unwrap();
        assert!(
            users.is_empty(),
            "migrate_database should not seed users (caller bootstraps admin)"
        );
    }

    #[tokio::test]
    async fn migrate_database_renames_legacy_after_copy() {
        let dir = TempDir::new().unwrap();

        let legacy_db = dir.path().join("assistant.db");
        let storage = crate::StorageLayer::new(&legacy_db).await.unwrap();
        drop(storage);

        // Sidecars may or may not exist after drop depending on WAL state;
        // create them explicitly so the test asserts removal regardless.
        std::fs::write(dir.path().join("assistant.db-wal"), b"fake wal").unwrap();
        std::fs::write(dir.path().join("assistant.db-shm"), b"fake shm").unwrap();

        migrate_database(dir.path()).await.unwrap();

        let space_db = dir.path().join("orgs/default/spaces/default/space.db");
        assert!(space_db.exists(), "space.db should exist after migration");

        let renamed = dir.path().join("assistant.db.legacy");
        assert!(
            renamed.exists(),
            "assistant.db should be renamed to assistant.db.legacy"
        );

        assert!(
            !legacy_db.exists(),
            "assistant.db should not exist after cutover"
        );
        assert!(
            !dir.path().join("assistant.db-wal").exists(),
            "assistant.db-wal sidecar should be removed"
        );
        assert!(
            !dir.path().join("assistant.db-shm").exists(),
            "assistant.db-shm sidecar should be removed"
        );
    }

    #[tokio::test]
    async fn migrate_database_failure_does_not_rename() {
        let dir = TempDir::new().unwrap();

        let legacy_db = dir.path().join("assistant.db");
        let storage = crate::StorageLayer::new(&legacy_db).await.unwrap();
        drop(storage);

        // Pre-create the destination as a directory so tokio::fs::copy fails
        // on write — simulates a broken target without exotic filesystems.
        let space_db_path = dir.path().join("orgs/default/spaces/default/space.db");
        std::fs::create_dir_all(&space_db_path).unwrap();

        let result = migrate_database(dir.path()).await;
        assert!(
            result.is_err(),
            "migrate_database should fail when space.db destination is unwritable"
        );

        assert!(
            legacy_db.exists(),
            "assistant.db must remain in place when migration fails"
        );
        assert!(
            !dir.path().join("assistant.db.legacy").exists(),
            "no rename should happen when migration fails"
        );
    }

    // -- full migration round-trip -------------------------------------------

    #[tokio::test]
    async fn full_migration_round_trip() {
        let dir = TempDir::new().unwrap();

        // Create a realistic legacy layout.
        // 1. Database with migrations applied.
        let legacy_db = dir.path().join("assistant.db");
        let storage = crate::StorageLayer::new(&legacy_db).await.unwrap();

        // Seed a conversation so we can verify it survives the migration.
        let conv_store = storage.conversation_store();
        let conv = conv_store
            .create_conversation(Some("Test Conversation"))
            .await
            .unwrap();
        let conv_id = conv.id;

        // Seed a persona.
        let persona_store = storage.persona_store();
        persona_store
            .create("test-persona", "Test Persona")
            .await
            .unwrap();

        // Drop the storage to release the pool (avoids locked db).
        drop(storage);

        // 2. Agents directory.
        std::fs::create_dir_all(dir.path().join("agents/default")).unwrap();
        std::fs::write(dir.path().join("agents/default/SOUL.md"), b"# Soul").unwrap();
        std::fs::write(dir.path().join("agents/default/IDENTITY.md"), b"# Identity").unwrap();

        // 3. Skills directory.
        std::fs::create_dir_all(dir.path().join("skills/greeting")).unwrap();
        std::fs::write(
            dir.path().join("skills/greeting/SKILL.md"),
            b"# Greeting Skill",
        )
        .unwrap();

        // 4. Config.
        std::fs::write(
            dir.path().join("config.toml"),
            "[llm]\nprovider = \"ollama\"\nmodel = \"qwen2.5:7b\"\n\n[storage]\n# db\n\n[memory]\n# enabled\n",
        )
        .unwrap();

        // Verify it's detected as legacy.
        assert!(is_legacy_layout(dir.path()), "should be legacy layout");

        // Compose the full migration the way the production caller does:
        //   1. backup    (assistant_backup::backup_legacy_install)
        //   2. fs        (storage::migration::migrate_filesystem)
        //   3. database  (storage::migration::migrate_database)
        //   4. admin     (assistant_auth::bootstrap::create_admin_user)
        let backup_path = assistant_backup::backup_legacy_install(dir.path())
            .await
            .unwrap();
        assert!(backup_path.exists(), "backup archive should exist");

        migrate_filesystem(dir.path()).await.unwrap();
        let (org_storage_for_bootstrap, org_id, space_id) =
            migrate_database(dir.path()).await.unwrap();
        let user_store = org_storage_for_bootstrap.user_store();
        let membership_store = org_storage_for_bootstrap.membership_store();
        assistant_auth::bootstrap::create_admin_user(
            &user_store,
            &membership_store,
            &org_id,
            &space_id,
        )
        .await
        .unwrap();
        drop(org_storage_for_bootstrap);

        // After migration, it should no longer be detected as legacy.
        assert!(
            !is_legacy_layout(dir.path()),
            "after migration, should NOT be detected as legacy (orgs/ exists now)"
        );

        // Verify the new structure.
        let space_dir = dir.path().join("orgs/default/spaces/default");

        // Files copied.
        assert!(space_dir.join("agents/default/SOUL.md").exists());
        assert!(space_dir.join("agents/default/IDENTITY.md").exists());
        assert!(space_dir.join("skills/greeting/SKILL.md").exists());

        // Config split.
        assert!(dir.path().join("server.toml").exists());
        assert!(dir.path().join("orgs/default/org.toml").exists());

        // space.db is a copy of assistant.db — open and verify data survived.
        let space_storage = crate::StorageLayer::new(&space_dir.join("space.db"))
            .await
            .unwrap();
        let found_conv = space_storage
            .conversation_store()
            .get_conversation(conv_id)
            .await
            .unwrap();
        assert!(
            found_conv.is_some(),
            "conversation should survive migration to space.db"
        );

        let personas = space_storage.persona_store().list().await.unwrap();
        assert!(
            personas.iter().any(|p| p.name == "Test Persona"),
            "persona should survive migration to space.db"
        );

        // org.db (at install root) has the admin user.
        let org_storage = crate::org_storage::OrgStorageLayer::new(&dir.path().join("org.db"))
            .await
            .unwrap();
        let orgs = org_storage.org_store().list_orgs().await.unwrap();
        let users = org_storage
            .user_store()
            .list_users(&orgs[0].id)
            .await
            .unwrap();
        assert_eq!(users.len(), 1, "admin user should exist in org.db");
        assert_eq!(users[0].email, "admin@localhost");
        assert!(
            !users[0].password_hash.is_empty(),
            "admin user should have a password hash"
        );

        // Verify admin has OrgAdmin membership.
        let spaces = org_storage
            .space_store()
            .list_spaces(&orgs[0].id)
            .await
            .unwrap();
        let members = org_storage
            .membership_store()
            .get_members_of_space(&spaces[0].id)
            .await
            .unwrap();
        assert_eq!(
            members.len(),
            1,
            "admin should be a member of the default space"
        );
        assert_eq!(
            members[0].role,
            assistant_core::identity::Role::OrgAdmin,
            "admin should have OrgAdmin role"
        );
    }
}
