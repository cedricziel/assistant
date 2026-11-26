//! `assistant persona` subcommands — list/create/use/skill-mode/timeout.

use std::path::Path;

use anyhow::Result;

use assistant_core::{default_workspace_dir, validate_agent_id};
use assistant_storage::{
    PersonaSkillAccessStore, PersonaStore, SqlitePersonaSkillAccessStore, SqlitePersonaStore,
    StorageLayer,
};

use crate::args::PersonaCommand;

pub async fn cmd_persona(
    install_root: &Path,
    db_path: &Path,
    command: &PersonaCommand,
) -> Result<()> {
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

            tokio::fs::create_dir_all(default_workspace_dir(install_root, id)).await?;
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
            tokio::fs::create_dir_all(default_workspace_dir(install_root, id)).await?;
            println!(
                "Persona '{}' activated. Use --agent {} on next run.",
                id, id
            );
        }
        PersonaCommand::SkillMode { persona_id, mode } => {
            let access_store = SqlitePersonaSkillAccessStore::new(storage.pool.clone());
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
            let persona_store = SqlitePersonaStore::new(storage.pool.clone());
            if persona_store.get(persona_id).await?.is_none() {
                anyhow::bail!("Persona '{}' not found", persona_id);
            }
            let access_store = SqlitePersonaSkillAccessStore::new(storage.pool.clone());
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
            let persona_store = SqlitePersonaStore::new(storage.pool.clone());
            if persona_store.get(persona_id).await?.is_none() {
                anyhow::bail!("Persona '{}' not found", persona_id);
            }
            let access_store = SqlitePersonaSkillAccessStore::new(storage.pool.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn db(root: &std::path::Path) -> PathBuf {
        root.join("test.db")
    }

    #[tokio::test]
    async fn list_runs_against_fresh_db() {
        let dir = tempfile::tempdir().unwrap();
        cmd_persona(dir.path(), &db(dir.path()), &PersonaCommand::List)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn skill_mode_set_succeeds_and_skill_add_then_remove_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let dbp = db(dir.path());

        // Set whitelist mode on default persona (ensure_exists is called at
        // the top of cmd_persona).
        cmd_persona(
            dir.path(),
            &dbp,
            &PersonaCommand::SkillMode {
                persona_id: "default".to_string(),
                mode: "whitelist".to_string(),
            },
        )
        .await
        .unwrap();

        cmd_persona(
            dir.path(),
            &dbp,
            &PersonaCommand::SkillAdd {
                persona_id: "default".to_string(),
                skill_name: "memory-get".to_string(),
            },
        )
        .await
        .unwrap();

        cmd_persona(
            dir.path(),
            &dbp,
            &PersonaCommand::SkillRemove {
                persona_id: "default".to_string(),
                skill_name: "memory-get".to_string(),
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn skill_remove_errors_for_unknown_persona() {
        let dir = tempfile::tempdir().unwrap();
        let res = cmd_persona(
            dir.path(),
            &db(dir.path()),
            &PersonaCommand::SkillRemove {
                persona_id: "no-such-persona".to_string(),
                skill_name: "anything".to_string(),
            },
        )
        .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn timeout_set_then_clear_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let dbp = db(dir.path());

        cmd_persona(
            dir.path(),
            &dbp,
            &PersonaCommand::TimeoutSet {
                persona_id: "default".to_string(),
                secs: 7200,
            },
        )
        .await
        .unwrap();

        cmd_persona(
            dir.path(),
            &dbp,
            &PersonaCommand::TimeoutClear {
                persona_id: "default".to_string(),
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn create_with_invalid_id_bails() {
        let dir = tempfile::tempdir().unwrap();
        let res = cmd_persona(
            dir.path(),
            &db(dir.path()),
            &PersonaCommand::Create {
                id: "bad id with spaces".to_string(),
            },
        )
        .await;
        let err = res.err().expect("must bail");
        assert!(err.to_string().contains("Invalid"));
    }

    #[tokio::test]
    async fn use_with_invalid_id_bails() {
        let dir = tempfile::tempdir().unwrap();
        let res = cmd_persona(
            dir.path(),
            &db(dir.path()),
            &PersonaCommand::Use {
                id: "bad id".to_string(),
            },
        )
        .await;
        let err = res.err().expect("must bail");
        assert!(err.to_string().contains("Invalid"));
    }
}
