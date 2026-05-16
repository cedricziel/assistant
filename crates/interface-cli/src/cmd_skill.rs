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
