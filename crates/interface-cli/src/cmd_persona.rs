//! `assistant persona` subcommands — list/create/use/skill-mode/timeout.

use std::path::Path;

use anyhow::Result;

use assistant_core::{default_workspace_dir, validate_agent_id};
use assistant_storage::{PersonaSkillAccessStore, PersonaStore, StorageLayer};

use crate::args::PersonaCommand;
use crate::home_agent_root;

pub async fn cmd_persona(db_path: &Path, command: &PersonaCommand) -> Result<()> {
    let storage = StorageLayer::new(db_path).await?;
    let store = storage.persona_store();
    store.ensure_exists("default").await?;

    match command {
        PersonaCommand::List => {
            let items = store.list().await?;
            if items.is_empty() {
                println!("No Personas configured.");
                return Ok(());
            }
            println!("Configured Personas:\n");
            for item in items {
                println!("  {:20} {}", item.id, item.name);
            }
        }
        PersonaCommand::Create { id } => {
            if !validate_agent_id(id) {
                anyhow::bail!(
                    "Invalid Persona ID '{}'. Use only letters, numbers, '-' and '_'.",
                    id
                );
            }
            store.ensure_exists(id).await?;

            let root = home_agent_root(id)?;
            let workspace = default_workspace_dir(id);
            tokio::fs::create_dir_all(&root).await?;
            tokio::fs::create_dir_all(&workspace).await?;
            println!("Persona '{}' is ready.", id);
        }
        PersonaCommand::Use { id } => {
            if !validate_agent_id(id) {
                anyhow::bail!(
                    "Invalid Persona ID '{}'. Use only letters, numbers, '-' and '_'.",
                    id
                );
            }
            store.ensure_exists(id).await?;
            let root = home_agent_root(id)?;
            let workspace = default_workspace_dir(id);
            tokio::fs::create_dir_all(&root).await?;
            tokio::fs::create_dir_all(&workspace).await?;
            println!(
                "Persona '{}' activated. Use --agent {} on next run.",
                id, id
            );
        }
        PersonaCommand::SkillMode { persona_id, mode } => {
            let access_store = PersonaSkillAccessStore::new(storage.pool.clone());
            if access_store.has_skill_list_entries(persona_id).await? {
                eprintln!(
                    "Warning: persona '{}' has existing skill list entries. \
                     Changing mode will reinterpret them as {} rules.",
                    persona_id, mode
                );
            }
            access_store.set_mode(persona_id, mode).await?;
            println!(
                "Persona '{}' skill access mode set to '{}'.",
                persona_id, mode
            );
        }
        PersonaCommand::SkillAdd {
            persona_id,
            skill_name,
        } => {
            let persona_store = PersonaStore::new(storage.pool.clone());
            if persona_store.get(persona_id).await?.is_none() {
                anyhow::bail!("Persona '{}' not found", persona_id);
            }
            let access_store = PersonaSkillAccessStore::new(storage.pool.clone());
            let mode = access_store.get_mode(persona_id).await?;
            if mode == "all" {
                eprintln!(
                    "Persona '{}' is in 'all' mode — use `skill-mode` to set whitelist or blacklist first.",
                    persona_id
                );
                std::process::exit(1);
            }
            access_store.add_skill(persona_id, skill_name).await?;
            println!(
                "Skill '{}' added to persona '{}' {} list.",
                skill_name, persona_id, mode
            );
        }
        PersonaCommand::SkillRemove {
            persona_id,
            skill_name,
        } => {
            let persona_store = PersonaStore::new(storage.pool.clone());
            if persona_store.get(persona_id).await?.is_none() {
                anyhow::bail!("Persona '{}' not found", persona_id);
            }
            let access_store = PersonaSkillAccessStore::new(storage.pool.clone());
            access_store.remove_skill(persona_id, skill_name).await?;
            println!(
                "Skill '{}' removed from persona '{}' list.",
                skill_name, persona_id
            );
        }
        PersonaCommand::TimeoutSet { persona_id, secs } => {
            store.set_turn_timeout(persona_id, *secs).await?;
            println!(
                "Persona '{}' turn timeout set to {} s ({:.1} h).",
                persona_id,
                secs,
                *secs as f64 / 3600.0
            );
        }
        PersonaCommand::TimeoutClear { persona_id } => {
            store.clear_turn_timeout(persona_id).await?;
            println!(
                "Persona '{}' turn timeout cleared (reverts to default 3 h).",
                persona_id
            );
        }
    }

    Ok(())
}
