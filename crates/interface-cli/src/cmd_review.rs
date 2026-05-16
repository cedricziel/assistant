//! `/review` REPL — accept or reject pending skill-refinement proposals.

use std::io::{self, Write as IoWrite};

use anyhow::Result;
use uuid::Uuid;

use assistant_storage::{RefinementStatus, StorageLayer, registry::SkillRegistry};

use crate::skill_diff;

pub async fn cmd_review(storage: &StorageLayer, registry: &SkillRegistry) -> Result<()> {
    let store = storage.refinements_store();
    let pending = store.list_by_status(&RefinementStatus::Pending).await?;

    if pending.is_empty() {
        println!("No pending skill refinement proposals.");
        return Ok(());
    }

    let use_colour = std::io::IsTerminal::is_terminal(&std::io::stdout());

    println!("\nPending skill refinement proposals:\n");
    for r in &pending {
        println!(
            "  id:     {}\n  skill:  {}\n  reason: {}",
            r.id, r.target_skill, r.rationale
        );

        // Read current SKILL.md from disk so the reviewer can see exactly
        // what the acceptance would change (#7).
        let skill_def = registry.get(&r.target_skill).await;
        let current_skill_md = match &skill_def {
            Some(def) => std::fs::read_to_string(def.dir.join("SKILL.md")).unwrap_or_default(),
            None => String::new(),
        };

        println!("  diff:");
        let diff =
            skill_diff::render_unified_diff(&current_skill_md, &r.proposed_skill_md, use_colour);
        for line in diff.lines() {
            println!("    {line}");
        }
        println!();
    }

    println!("Commands: accept <id>  |  reject <id> [note]  |  done");

    loop {
        print!("review> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();

        if line.is_empty() || line == "done" || line == "q" {
            break;
        }

        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        match parts.as_slice() {
            ["accept", id_str] => {
                let id = match Uuid::parse_str(id_str) {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("Invalid UUID: {id_str}");
                        continue;
                    }
                };

                // Find the refinement in the pending list.
                let Some(refinement) = pending.iter().find(|r| r.id == id) else {
                    eprintln!("Refinement {id} not found in pending list.");
                    continue;
                };

                // Find the skill in the registry to get its directory.
                let skill_def = registry.get(&refinement.target_skill).await;
                if let Some(def) = skill_def {
                    let skill_md_path = def.dir.join("SKILL.md");
                    if let Err(e) = std::fs::write(&skill_md_path, &refinement.proposed_skill_md) {
                        eprintln!("Failed to write SKILL.md: {e}");
                        continue;
                    }
                    // Reload the skill from disk.
                    if let Err(e) = registry.reload(&refinement.target_skill).await {
                        eprintln!("Failed to reload skill: {e}");
                    } else {
                        println!("Skill '{}' updated and reloaded.", refinement.target_skill);
                    }
                } else {
                    eprintln!(
                        "Skill '{}' not found in registry; cannot write SKILL.md.",
                        refinement.target_skill
                    );
                }

                store.review(id, true, None).await?;
                println!("Refinement {id} accepted.");
            }

            ["reject", id_str] => {
                let id = match Uuid::parse_str(id_str) {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("Invalid UUID: {id_str}");
                        continue;
                    }
                };
                store.review(id, false, None).await?;
                println!("Refinement {id} rejected.");
            }

            ["reject", id_str, note] => {
                let id = match Uuid::parse_str(id_str) {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("Invalid UUID: {id_str}");
                        continue;
                    }
                };
                store.review(id, false, Some(note)).await?;
                println!("Refinement {id} rejected with note.");
            }

            _ => {
                eprintln!("Unknown command. Use: accept <id> | reject <id> [note] | done");
            }
        }
    }

    Ok(())
}
