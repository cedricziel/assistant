//! Skills published by external MCP servers through the Skills extension.
//!
//! A remote skill is untrusted. [`McpSkillSource`] offers one only after every
//! file the manifest lists has been fetched with `resources/read` and has the
//! size and sha256 digest the manifest promises, and after `SKILL.md`'s
//! frontmatter agrees with the manifest. Verified files are kept under
//! `{base}/cache/mcp-skills/{server}/{skill}/`, next to a copy of the manifest
//! they were verified against. A skill is read from that cache only while the
//! server's manifest is unchanged and the files on disk still match it;
//! otherwise it is fetched and verified again.
//!
//! A skill whose manifest says `"dynamic"` lists no files, so there is nothing
//! to check them against. Its `SKILL.md` is still fetched, and its frontmatter
//! still has to match, but its size and digest are not checked and it is never
//! served from the cache.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use tracing::{debug, warn};

use assistant_skills::{SkillDef, SkillSource, parse_skill_content};
use rmcp::model::{SKILL_MD, Skill, SkillResources};

use crate::client::McpClient;

/// One file of a skill: its path inside the skill and the URI to fetch it from.
struct Entry<'a> {
    relative: PathBuf,
    uri: &'a str,
}

/// Lists, verifies and caches the skills of MCP servers.
pub struct McpSkillSource {
    cache_root: PathBuf,
}

impl McpSkillSource {
    /// A source that caches under `{base_dir}/cache/mcp-skills`.
    pub fn new(base_dir: &Path) -> Self {
        Self {
            cache_root: base_dir.join("cache").join("mcp-skills"),
        }
    }

    /// The skills `client`'s server publishes that pass verification, as
    /// `{server}__{skill}`. Local skills are kebab-case, which has no
    /// underscores, so a remote skill can never replace one.
    pub async fn skills(&self, client: &McpClient) -> Vec<SkillDef> {
        let server = client.name();
        if !client.supports_skills() {
            return Vec::new();
        }
        let listed = match client.list_skills().await {
            Ok(listed) => listed,
            Err(error) => {
                warn!(%server, error = %format!("{error:#}"), "could not list the server's skills");
                return Vec::new();
            }
        };
        let mut offered = Vec::new();
        for skill in &listed {
            match self.verified(client, skill).await {
                Ok(def) => offered.push(def),
                Err(error) => {
                    warn!(
                        %server,
                        skill = skill.name(),
                        error = %format!("{error:#}"),
                        "remote skill rejected"
                    );
                    self.forget(server, skill.name());
                }
            }
        }
        offered
    }

    async fn verified(&self, client: &McpClient, skill: &Skill) -> Result<SkillDef> {
        let server = client.name();
        let name = skill.name();
        skill.validate().context("invalid manifest")?;
        let dir = self.skill_dir(server, name)?;
        let entries = entries(skill)?;
        let manifest = serde_json::to_string(skill).context("manifest is not serialisable")?;

        let (files, fresh) = match self.cached(server, name, &manifest, skill, &entries) {
            Some(files) => (files, false),
            None => (fetch(client, skill, &entries).await?, true),
        };

        let skill_md = files
            .iter()
            .find(|(relative, _)| relative == Path::new(SKILL_MD))
            .context("SKILL.md was not fetched")?;
        let text = std::str::from_utf8(&skill_md.1).context("SKILL.md is not UTF-8")?;
        let mut def = parse_skill_content(
            text,
            &dir,
            SkillSource::Mcp {
                server: server.to_string(),
            },
        )
        .context("SKILL.md does not parse")?;
        if def.name != name || def.description != skill.frontmatter.description.trim() {
            bail!("SKILL.md's frontmatter does not match the manifest");
        }

        if fresh {
            self.store(server, name, &dir, &files, &manifest)?;
        }
        def.name = format!("{server}__{name}");
        Ok(def)
    }

    /// The cached files of `skill`, if its manifest is the one they were
    /// verified against and they still have the promised size and digest.
    fn cached(
        &self,
        server: &str,
        name: &str,
        manifest: &str,
        skill: &Skill,
        entries: &[Entry<'_>],
    ) -> Option<Vec<(PathBuf, Vec<u8>)>> {
        let SkillResources::Files(resources) = &skill.resources else {
            return None;
        };
        if std::fs::read_to_string(self.manifest_path(server, name)).ok()? != manifest {
            return None;
        }
        let dir = self.skill_dir(server, name).ok()?;
        entries
            .iter()
            .map(|entry| {
                let bytes = std::fs::read(dir.join(&entry.relative)).ok()?;
                let resource = resources.iter().find(|r| r.uri == entry.uri)?;
                resource.verify(&bytes).ok()?;
                Some((entry.relative.clone(), bytes))
            })
            .collect()
    }

    /// Replace the cache of a skill with `files`, manifest last.
    fn store(
        &self,
        server: &str,
        name: &str,
        dir: &Path,
        files: &[(PathBuf, Vec<u8>)],
        manifest: &str,
    ) -> Result<()> {
        self.forget(server, name);
        std::fs::create_dir_all(dir).context("cannot create the cache directory")?;
        let root = dir.canonicalize()?;
        for (relative, bytes) in files {
            let target = root.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
                if !parent.canonicalize()?.starts_with(&root) {
                    bail!("{} leaves the skill directory", relative.display());
                }
            }
            std::fs::write(&target, bytes)
                .with_context(|| format!("cannot write {}", relative.display()))?;
        }
        std::fs::write(self.manifest_path(server, name), manifest)
            .context("cannot write the manifest")?;
        Ok(())
    }

    /// Drop everything cached for a skill.
    fn forget(&self, server: &str, name: &str) {
        let (Ok(dir), manifest) = (
            self.skill_dir(server, name),
            self.manifest_path(server, name),
        ) else {
            return;
        };
        for result in [
            std::fs::remove_dir_all(&dir),
            std::fs::remove_file(&manifest),
        ] {
            if let Err(error) = result
                && error.kind() != std::io::ErrorKind::NotFound
            {
                debug!(%error, "could not clear the skill cache");
            }
        }
    }

    fn skill_dir(&self, server: &str, name: &str) -> Result<PathBuf> {
        Ok(self.cache_root.join(segment(server)).join(safe_name(name)?))
    }

    fn manifest_path(&self, server: &str, name: &str) -> PathBuf {
        self.cache_root
            .join(segment(server))
            .join(format!("{name}.manifest.json"))
    }
}

/// Fetch the files of `skill` and, when the manifest lists them, check each.
async fn fetch(
    client: &McpClient,
    skill: &Skill,
    entries: &[Entry<'_>],
) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut files = Vec::with_capacity(entries.len());
    for entry in entries {
        let bytes = client.read_resource_bytes(entry.uri).await?;
        if let SkillResources::Files(resources) = &skill.resources
            && let Some(resource) = resources.iter().find(|r| r.uri == entry.uri)
        {
            resource.verify(&bytes)?;
        }
        files.push((entry.relative.clone(), bytes));
    }
    Ok(files)
}

/// The files to fetch for `skill`, each with a path that stays inside the
/// skill's directory. A `"dynamic"` skill has `SKILL.md` alone.
fn entries(skill: &Skill) -> Result<Vec<Entry<'_>>> {
    let SkillResources::Files(resources) = &skill.resources else {
        return Ok(vec![Entry {
            relative: PathBuf::from(SKILL_MD),
            uri: &skill.uri,
        }]);
    };
    let root = skill
        .uri
        .strip_suffix(SKILL_MD)
        .context("the skill URI does not end in SKILL.md")?;
    resources
        .iter()
        .map(|resource| {
            Ok(Entry {
                relative: relative_path(root, &resource.uri)?,
                uri: &resource.uri,
            })
        })
        .collect()
}

/// The path of `uri` below the skill's directory URI `root`.
///
/// Every segment must be a plain name: `..`, `.`, empty segments, separators
/// and drive prefixes are refused, so a manifest cannot point outside the
/// skill's cache directory. Segments are used as they are, not
/// percent-decoded.
fn relative_path(root: &str, uri: &str) -> Result<PathBuf> {
    let relative = uri
        .strip_prefix(root)
        .with_context(|| format!("{uri} is not inside the skill's directory"))?;
    let mut path = PathBuf::new();
    for part in relative.split('/') {
        let mut components = Path::new(part).components();
        let plain = matches!(
            (components.next(), components.next()),
            (Some(Component::Normal(_)), None)
        );
        if !plain || part.contains(['\\', '?', '#', '\0']) {
            bail!("{uri} is not a plain path inside the skill's directory");
        }
        path.push(part);
    }
    Ok(path)
}

/// `name` as a single directory name: lowercase ASCII letters, digits and hyphens.
fn safe_name(name: &str) -> Result<&str> {
    if !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        Ok(name)
    } else {
        bail!("'{name}' is not a plain kebab-case skill name")
    }
}

/// A server name as a single directory name.
fn segment(server: &str) -> String {
    server
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "skill://acme/deploy/";

    #[test]
    fn nested_files_keep_their_relative_path() {
        assert_eq!(
            relative_path(ROOT, "skill://acme/deploy/scripts/run.sh").unwrap(),
            Path::new("scripts/run.sh")
        );
    }

    #[test]
    fn a_manifest_entry_that_climbs_out_of_the_skill_is_refused() {
        for uri in [
            "skill://acme/deploy/../../evil.sh",
            "skill://acme/deploy/scripts/../../evil.sh",
            "skill://acme/deploy/./run.sh",
            "skill://acme/deploy//etc/passwd",
            "skill://acme/deploy/scripts\\..\\evil.sh",
            "skill://acme/other/run.sh",
        ] {
            assert!(
                relative_path(ROOT, uri).is_err(),
                "{uri} must not resolve inside the skill"
            );
        }
    }

    #[test]
    fn a_server_name_cannot_escape_the_cache() {
        assert_eq!(segment("../../etc"), "______etc");
        assert_eq!(segment("my.server"), "my_server");
    }

    #[test]
    fn a_skill_name_must_be_a_plain_kebab_case_segment() {
        assert!(safe_name("deploy-2").is_ok());
        for name in ["", "..", "a/b", "Deploy", "a_b", "dépôt"] {
            assert!(safe_name(name).is_err(), "'{name}' must be refused");
        }
    }
}
