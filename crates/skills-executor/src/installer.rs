//! Skill installation from local paths or GitHub repositories.
//!
//! Supports two source formats:
//! * Local path  — absolute (`/some/dir`), home-relative (`~/skills/foo`), or relative (`./foo`)
//! * GitHub      — `owner/repo` or `owner/repo/sub/path` (fetches the `SKILL.md` via raw GitHub)

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use assistant_core::{parser::parse_skill_dir, skill::SkillSource};
use assistant_storage::SkillRegistry;
use tracing::{debug, info};

/// Install a skill from `source` into `skills_dir` and register it in `registry`.
///
/// Returns the installed skill's name on success.
pub async fn install_skill_from_source(
    source: &str,
    skills_dir: &Path,
    registry: Arc<SkillRegistry>,
) -> Result<String> {
    let source = source.trim();

    if is_local_path(source) {
        install_from_local(source, skills_dir, registry).await
    } else {
        install_from_github(source, skills_dir, registry).await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_local_path(s: &str) -> bool {
    s.starts_with('/') || s.starts_with("~/") || s.starts_with("./") || s.starts_with("../")
}

async fn install_from_local(
    source: &str,
    skills_dir: &Path,
    registry: Arc<SkillRegistry>,
) -> Result<String> {
    let src_path = expand_tilde(source);

    if !src_path.exists() {
        return Err(anyhow!("Path '{}' does not exist", src_path.display()));
    }
    if !src_path.is_dir() {
        return Err(anyhow!("Path '{}' is not a directory", src_path.display()));
    }
    let skill_md = src_path.join("SKILL.md");
    if !skill_md.exists() {
        return Err(anyhow!(
            "No SKILL.md found in '{}'",
            src_path.display()
        ));
    }

    // Parse to get the name
    let def = parse_skill_dir(&src_path, SkillSource::User)
        .with_context(|| format!("Failed to parse SKILL.md in '{}'", src_path.display()))?;

    let dest = skills_dir.join(&def.name);
    if dest != src_path {
        // Copy the skill directory into the user skills dir
        copy_dir_all(&src_path, &dest)
            .with_context(|| format!("Failed to copy skill to '{}'", dest.display()))?;
        info!(name = %def.name, dest = %dest.display(), "Copied skill directory");
    }

    // Parse from the destination (canonical path)
    let installed_def = parse_skill_dir(&dest, SkillSource::User)
        .with_context(|| format!("Failed to parse installed skill at '{}'", dest.display()))?;

    let name = installed_def.name.clone();
    registry.register(installed_def).await?;
    info!(name = %name, "Installed skill from local path");
    Ok(name)
}

async fn install_from_github(
    source: &str,
    skills_dir: &Path,
    registry: Arc<SkillRegistry>,
) -> Result<String> {
    // Parse `owner/repo[/sub/path]`
    let parts: Vec<&str> = source.splitn(3, '/').collect();
    if parts.len() < 2 {
        return Err(anyhow!(
            "Invalid GitHub source '{}'. Expected 'owner/repo' or 'owner/repo/path'",
            source
        ));
    }

    let owner = parts[0];
    let repo = parts[1];
    let sub_path = if parts.len() == 3 { parts[2] } else { "" };

    // Fetch SKILL.md from the default branch
    let skill_md_url = if sub_path.is_empty() {
        format!(
            "https://raw.githubusercontent.com/{owner}/{repo}/main/SKILL.md"
        )
    } else {
        format!(
            "https://raw.githubusercontent.com/{owner}/{repo}/main/{sub_path}/SKILL.md"
        )
    };

    debug!(url = %skill_md_url, "Fetching SKILL.md from GitHub");

    let client = reqwest::Client::builder()
        .user_agent("assistant-skill-installer/0.1")
        .build()?;

    let resp = client
        .get(&skill_md_url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch '{skill_md_url}'"))?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "GitHub returned HTTP {} for '{skill_md_url}'",
            resp.status()
        ));
    }

    let skill_md_content = resp
        .text()
        .await
        .context("Failed to read response body")?;

    // Parse just the frontmatter to get the skill name
    let temp_def = assistant_core::parser::parse_skill_content(&skill_md_content, skills_dir, SkillSource::User)
        .context("Failed to parse fetched SKILL.md")?;

    let skill_name = temp_def.name.clone();
    let dest = skills_dir.join(&skill_name);

    // Create the destination directory and write SKILL.md
    tokio::fs::create_dir_all(&dest)
        .await
        .with_context(|| format!("Failed to create skill directory '{}'", dest.display()))?;

    let skill_md_dest = dest.join("SKILL.md");
    tokio::fs::write(&skill_md_dest, &skill_md_content)
        .await
        .with_context(|| format!("Failed to write '{}'", skill_md_dest.display()))?;

    // Parse from the real destination path
    let installed_def = parse_skill_dir(&dest, SkillSource::User)
        .with_context(|| format!("Failed to parse installed skill at '{}'", dest.display()))?;

    let name = installed_def.name.clone();
    registry.register(installed_def).await?;
    info!(name = %name, source = %source, "Installed skill from GitHub");
    Ok(name)
}

/// Recursively copy a directory tree.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

/// Expand a leading `~/` to the user's home directory.
fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}
