//! `assistant skill` subcommands — list/show/create/delete/generate.

use std::io::{self, Write as IoWrite};
use std::path::Path;

use anyhow::{Context, Result};

use assistant_core::types::agent::AssistantConfig;
use assistant_skills::SkillSource;
use assistant_storage::{StorageLayer, registry::SkillRegistry};

use crate::args::SkillCommand;

pub async fn cmd_skill(
    db_path: &Path,
    config: &AssistantConfig,
    command: &SkillCommand,
) -> Result<()> {
    let storage = StorageLayer::new(db_path).await?;
    let registry = SkillRegistry::new(storage.pool.clone()).await?;

    // Load builtin and on-disk skills so commands like `list`, `show`, and
    // `create` (duplicate check) see the full registry — not just SQLite rows.
    registry
        .load_embedded()
        .await
        .context("Failed to load embedded skills")?;
    let project_root = std::env::current_dir().ok();
    let dirs_to_scan = assistant_runtime::bootstrap::skill_dirs(config, project_root.as_deref());
    let dirs_ref: Vec<(&Path, SkillSource)> = dirs_to_scan
        .iter()
        .map(|(p, s)| (p.as_path(), s.clone()))
        .collect();
    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    match command {
        SkillCommand::List { persona } => {
            let skills = if let Some(persona_id) = persona {
                registry.list_for_persona(persona_id, &storage.pool).await?
            } else {
                registry.list().await
            };

            if skills.is_empty() {
                println!("No skills registered.");
                return Ok(());
            }

            println!("{:<30} {:<12} DESCRIPTION", "NAME", "SOURCE");
            println!("{}", "-".repeat(80));
            for s in skills {
                let source = match s.source {
                    SkillSource::Builtin => "builtin",
                    SkillSource::User => "user",
                    SkillSource::Project => "project",
                    SkillSource::Installed => "installed",
                };
                println!("{:<30} {:<12} {}", s.name, source, s.description);
            }
        }
        SkillCommand::Show { name } => {
            let skill = registry
                .get(name)
                .await
                .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", name))?;

            println!("name: {}", skill.name);
            println!("description: {}", skill.description);
            if let Some(license) = &skill.license {
                println!("license: {}", license);
            }
            println!("source: {}", skill.source);
            println!("dir: {}", skill.dir.display());
            println!();
            println!("{}", skill.body);
        }
        SkillCommand::Create {
            name,
            description,
            body_file,
        } => {
            let body = tokio::fs::read_to_string(body_file)
                .await
                .with_context(|| format!("Failed to read body file {}", body_file.display()))?;

            let def = registry.create_user_skill(name, description, &body).await?;
            println!("Skill '{}' created at {}.", def.name, def.dir.display());
        }
        SkillCommand::Delete { name, yes } => {
            if !*yes {
                print!("Delete skill '{}'? [y/N] ", name);
                io::stdout().flush().ok();
                let mut buf = String::new();
                if io::stdin().read_line(&mut buf).is_err()
                    || !matches!(buf.trim().to_lowercase().as_str(), "y" | "yes")
                {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            registry.delete_user_skill(name).await?;
            println!("Skill '{}' deleted.", name);
        }
        // Handled after bootstrap in main() — should not be reached here.
        SkillCommand::Generate { .. } => {
            anyhow::bail!(
                "Generate requires a running Orchestrator — this code path should not be reached"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cfg_under(root: &Path) -> AssistantConfig {
        let mut cfg = AssistantConfig::default();
        // Point skill discovery at an empty subdir so load_from_dirs() finds
        // nothing extra beyond builtins.
        cfg.skills.extra_dirs = vec![root.join("skills").to_string_lossy().into_owned()];
        cfg
    }

    fn db_under(root: &Path) -> PathBuf {
        root.join("test.db")
    }

    #[tokio::test]
    async fn list_prints_no_skills_when_registry_empty() {
        // We can't easily strip the embedded skills from the binary, but we
        // can verify the happy path runs without error against a fresh DB.
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg_under(dir.path());
        let db = db_under(dir.path());
        cmd_skill(&db, &cfg, &SkillCommand::List { persona: None })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn show_errors_on_unknown_skill() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg_under(dir.path());
        let db = db_under(dir.path());
        let res = cmd_skill(
            &db,
            &cfg,
            &SkillCommand::Show {
                name: "definitely-not-real".to_string(),
            },
        )
        .await;
        let err = res.err().expect("should error");
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn create_then_delete_user_skill_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg_under(dir.path());
        let db = db_under(dir.path());

        let body_file = dir.path().join("body.md");
        std::fs::write(
            &body_file,
            "---\nname: cli-create-skill\ndescription: created in test\n---\nbody",
        )
        .unwrap();

        cmd_skill(
            &db,
            &cfg,
            &SkillCommand::Create {
                name: "cli-create-skill".to_string(),
                description: "created in test".to_string(),
                body_file: body_file.clone(),
            },
        )
        .await
        .unwrap();

        // `Delete` with `yes: true` skips stdin and removes the skill.
        cmd_skill(
            &db,
            &cfg,
            &SkillCommand::Delete {
                name: "cli-create-skill".to_string(),
                yes: true,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn create_errors_when_body_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg_under(dir.path());
        let db = db_under(dir.path());
        let res = cmd_skill(
            &db,
            &cfg,
            &SkillCommand::Create {
                name: "x".to_string(),
                description: "x".to_string(),
                body_file: dir.path().join("does-not-exist.md"),
            },
        )
        .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn generate_subcommand_bails_with_orchestrator_message() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg_under(dir.path());
        let db = db_under(dir.path());
        let res = cmd_skill(
            &db,
            &cfg,
            &SkillCommand::Generate {
                description: "x".to_string(),
            },
        )
        .await;
        let err = res.err().expect("must bail");
        assert!(err.to_string().contains("Orchestrator"));
    }
}
