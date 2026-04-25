//! Legacy-to-multi-org migration for existing single-user installations.
//!
//! Detects the legacy `~/.assistant/` layout (flat `assistant.db` + `agents/` +
//! `skills/` + `config.toml`) and migrates it to the new org/space directory
//! structure introduced by the multi-user feature.
//!
//! ## Migration steps
//!
//! 1. **Detect** — [`is_legacy_layout`] checks for `assistant.db` without `orgs/`.
//! 2. **Backup** — [`backup_legacy`] creates a `.tar.gz` snapshot via the backup crate.
//! 3. **Filesystem** — [`migrate_filesystem`] creates `orgs/default/spaces/default/`
//!    and copies `agents/`, `skills/`, interface configs.
//! 4. **Database** — [`migrate_database`] copies `assistant.db` → `space.db`, creates
//!    `org.db` with the default org, space, and initial admin user.
//! 5. **Admin user** — [`create_admin_user`] seeds the first admin account.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::info;

use assistant_backup::{BackupEngine, BackupOptions};

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

// -- Backup ------------------------------------------------------------------

/// Create a `.tar.gz` backup of the entire legacy installation before migrating.
///
/// Returns the path to the created archive. Uses the existing [`BackupEngine`]
/// so all discovery, WAL checkpoint, and manifest logic is reused.
pub async fn backup_legacy(base_path: &Path) -> Result<PathBuf> {
    let backups_dir = base_path.join("backups");
    tokio::fs::create_dir_all(&backups_dir)
        .await
        .with_context(|| format!("creating backups directory: {}", backups_dir.display()))?;

    let archive_name = format!(
        "pre-migration-{}.tar.gz",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let output_path = backups_dir.join(&archive_name);

    let opts = BackupOptions {
        install_dir: base_path.to_path_buf(),
        output_path: output_path.clone(),
        db_path: Some(base_path.join("assistant.db")),
    };

    let result = BackupEngine::new()
        .run(opts)
        .await
        .context("creating pre-migration backup")?;

    info!(
        "pre-migration backup created: {} ({} files, {} bytes)",
        result.output_path.display(),
        result.entry_count,
        result.archive_size
    );

    Ok(result.output_path)
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
            // Lines before the first section header go to both (comments/preamble).
            server_lines.push(line.to_string());
            org_lines.push(line.to_string());
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

/// Copy `assistant.db` to `space.db` and create `org.db` with the default
/// org, space, and initial admin user.
///
/// - `assistant.db` → `orgs/default/spaces/default/space.db` (verbatim copy;
///   existing space-level migrations will apply on next startup).
/// - Creates `orgs/default/org.db` with org schema populated with:
///   - Default organization row
///   - Default space row
///   - Admin user (see [`create_admin_user`])
///   - Admin's space membership (OrgAdmin role)
pub async fn migrate_database(base_path: &Path) -> Result<()> {
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

    // Also copy WAL/SHM if present
    for suffix in &["-wal", "-shm"] {
        let src = base_path.join(format!("assistant.db{suffix}"));
        if src.exists() {
            let dst = space_dir.join(format!("space.db{suffix}"));
            tokio::fs::copy(&src, &dst).await.ok();
        }
    }

    // Create org.db via OrgStorageLayer (runs org migrations).
    let org_db_path = org_dir.join("org.db");
    let org_storage = crate::org_storage::OrgStorageLayer::new(&org_db_path)
        .await
        .context("creating org.db")?;

    // Seed default org.
    use assistant_core::identity::{OrgId, SpaceId};
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

    // Create admin user and membership.
    let admin_user_id = create_admin_user(&org_storage, &org.id.0, &space.id.0).await?;
    info!("created admin user: {admin_user_id}");

    Ok(())
}

// -- Admin user creation -----------------------------------------------------

/// Create the initial admin user during migration.
///
/// - If `ASSISTANT_WEB_TOKEN` is set, uses it as a temporary password.
/// - Otherwise, generates a random 24-character password and prints it to stdout.
///
/// Returns the new user's ID.
pub async fn create_admin_user(
    org_storage: &crate::org_storage::OrgStorageLayer,
    org_id: &str,
    space_id: &str,
) -> Result<String> {
    use assistant_auth::password::hash_password;
    use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
    use assistant_core::store::{MembershipStore, SpaceMembership, User, UserStore};
    use rand::Rng;

    let now = chrono::Utc::now();

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

    let user_id = format!("usr_{}", uuid::Uuid::new_v4());
    let user = User {
        id: UserId::from(user_id.clone()),
        org_id: OrgId::from(org_id),
        email: "admin@localhost".into(),
        name: "Admin".into(),
        password_hash,
        idp_issuer: None,
        idp_subject: None,
        created_at: now,
        updated_at: now,
    };

    org_storage
        .user_store()
        .create_user(&user)
        .await
        .context("creating admin user")?;

    // Grant OrgAdmin role in default space.
    let membership = SpaceMembership {
        user_id: UserId::from(user_id.clone()),
        space_id: SpaceId::from(space_id),
        role: Role::OrgAdmin,
        created_at: now,
    };
    org_storage
        .membership_store()
        .add_membership(&membership)
        .await
        .context("granting admin membership")?;

    if was_generated {
        // Print to stdout so the operator sees it exactly once.
        println!();
        println!("═══════════════════════════════════════════════════════════");
        println!("  Migration complete — initial admin credentials:");
        println!();
        println!("    Email:    admin@localhost");
        println!("    Password: {password}");
        println!();
        println!("  Change this password after first login.");
        println!("═══════════════════════════════════════════════════════════");
        println!();
    } else {
        println!();
        println!("══════════════���════════════════════════════════════════════");
        println!("  Migration complete — admin user created.");
        println!();
        println!("    Email:    admin@localhost");
        println!("    Password: (your ASSISTANT_WEB_TOKEN value)");
        println!("═════���═════════════════════════════════��═══════════════════");
        println!();
    }

    Ok(user_id)
}

// -- Full migration entry point ----------------------------------------------

/// Run the complete legacy → multi-org migration.
///
/// 1. Detect legacy layout (caller should check [`is_legacy_layout`] first).
/// 2. Create a pre-migration backup.
/// 3. Migrate filesystem layout.
/// 4. Migrate database (copy + create org.db with admin user).
///
/// Returns the path to the pre-migration backup archive.
pub async fn run_migration(base_path: &Path) -> Result<PathBuf> {
    info!(
        "starting legacy → multi-org migration at {}",
        base_path.display()
    );

    // Step 1: Backup
    let backup_path = backup_legacy(base_path).await?;

    // Step 2: Filesystem migration (dirs, config split)
    migrate_filesystem(base_path).await?;

    // Step 3: Database migration (copy + org.db creation)
    migrate_database(base_path).await?;

    info!("migration complete. Backup at: {}", backup_path.display());
    Ok(backup_path)
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

    // -- backup_legacy tests -------------------------------------------------

    #[tokio::test]
    async fn backup_legacy_creates_archive() {
        let dir = TempDir::new().unwrap();

        // Seed a minimal legacy layout.
        std::fs::write(dir.path().join("assistant.db"), b"fake-db-content").unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            b"[llm]\nprovider = \"ollama\"",
        )
        .unwrap();

        let archive_path = backup_legacy(dir.path()).await.unwrap();

        assert!(archive_path.exists(), "backup archive should be created");
        assert!(
            archive_path.to_string_lossy().contains("pre-migration"),
            "archive name should contain 'pre-migration'"
        );
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

        // Create a valid legacy database with migrations.
        let legacy_db = dir.path().join("assistant.db");
        let _storage = crate::StorageLayer::new(&legacy_db).await.unwrap();

        migrate_database(dir.path()).await.unwrap();

        let space_db = dir.path().join("orgs/default/spaces/default/space.db");
        assert!(
            space_db.exists(),
            "space.db should be created by copying assistant.db"
        );

        let org_db = dir.path().join("orgs/default/org.db");
        assert!(org_db.exists(), "org.db should be created with org schema");

        // Verify org.db has a default organization.
        let org_storage = crate::org_storage::OrgStorageLayer::new(&org_db)
            .await
            .unwrap();
        let orgs = org_storage.org_store().list_orgs().await.unwrap();
        assert_eq!(orgs.len(), 1, "org.db should have exactly one organization");
        assert_eq!(orgs[0].slug, "default");

        // Verify there's a default space.
        let spaces = org_storage
            .space_store()
            .list_spaces(&orgs[0].id)
            .await
            .unwrap();
        assert_eq!(spaces.len(), 1, "org.db should have exactly one space");
        assert_eq!(spaces[0].slug, "default");

        // Verify admin user exists.
        let users = org_storage
            .user_store()
            .list_users(&orgs[0].id)
            .await
            .unwrap();
        assert_eq!(
            users.len(),
            1,
            "org.db should have exactly one user (admin)"
        );
        assert_eq!(users[0].email, "admin@localhost");
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

        // Run full migration.
        let backup_path = run_migration(dir.path()).await.unwrap();
        assert!(backup_path.exists(), "backup archive should exist");

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

        // org.db has the admin user.
        let org_storage =
            crate::org_storage::OrgStorageLayer::new(&dir.path().join("orgs/default/org.db"))
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
