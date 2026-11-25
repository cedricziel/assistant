//! Configuration loading for the assistant.
//!
//! Settings live in two kinds of files under the install root
//! (`~/.assistant`):
//!
//! * `server.toml` — process-wide settings ([`ServerConfig`]), loaded once at
//!   startup.
//! * `orgs/{slug}/org.toml` — one file per organization ([`OrgConfig`]), loaded
//!   for the org a space belongs to.
//!
//! A missing file yields the defaults; an unreadable or malformed file is an
//! error, so a typo never silently falls back to the defaults.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tracing::info;

use crate::types::agent::{OrgConfig, ServerConfig};

/// Path of `server.toml` under the install root `base_dir`.
pub fn server_config_path(base_dir: &Path) -> PathBuf {
    base_dir.join("server.toml")
}

/// Path of `orgs/{org_slug}/org.toml` under the install root `base_dir`.
///
/// Rejects slugs that could leave the `orgs/` directory.
pub fn org_config_path(base_dir: &Path, org_slug: &str) -> Result<PathBuf> {
    if org_slug.is_empty()
        || !org_slug
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!(
            "org slug {org_slug:?} must be non-empty and contain only letters, digits, '-' and '_'"
        );
    }
    Ok(base_dir.join("orgs").join(org_slug).join("org.toml"))
}

/// Load `server.toml` from the install root `base_dir`.
///
/// Fails when an MCP server is named with a prefix reserved for integration
/// tools (see [`RESERVED_MCP_SERVER_PREFIXES`](crate::types::skills_mcp::RESERVED_MCP_SERVER_PREFIXES)).
pub fn load_server_config(base_dir: &Path) -> Result<ServerConfig> {
    let path = server_config_path(base_dir);
    let config: ServerConfig = load_toml(&path)?;
    config
        .mcp
        .check_names()
        .with_context(|| format!("Invalid [[mcp.servers]] in {}", path.display()))?;
    Ok(config)
}

/// Load `orgs/{org_slug}/org.toml` from the install root `base_dir`.
pub fn load_org_config(base_dir: &Path, org_slug: &str) -> Result<OrgConfig> {
    load_toml(&org_config_path(base_dir, org_slug)?)
}

/// Read and parse `path`, or return `T::default()` when the file is absent.
///
/// Synchronous on purpose: config loads at startup (or once per org) from a
/// tiny file, and this keeps the loaders usable without a tokio runtime.
fn load_toml<T: Default + serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    if !path.exists() {
        info!("No config file at {}; using defaults", path.display());
        return Ok(T::default());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let parsed =
        toml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;
    info!("Loaded config from {}", path.display());
    Ok(parsed)
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::llm::LlmProviderKind;
    use crate::types::storage::BusKind;

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn paths_follow_the_documented_layout() {
        let base = Path::new("/home/u/.assistant");
        assert_eq!(
            server_config_path(base),
            Path::new("/home/u/.assistant/server.toml")
        );
        assert_eq!(
            org_config_path(base, "acme").unwrap(),
            Path::new("/home/u/.assistant/orgs/acme/org.toml")
        );
    }

    #[test]
    fn org_config_path_rejects_slugs_that_escape_the_orgs_directory() {
        let base = Path::new("/home/u/.assistant");
        for slug in ["", "..", "../x", "a/b", "a b"] {
            assert!(org_config_path(base, slug).is_err(), "slug {slug:?}");
        }
    }

    #[test]
    fn missing_files_yield_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let server = load_server_config(dir.path()).unwrap();
        assert_eq!(server.bus.kind, BusKind::Sqlite);
        let org = load_org_config(dir.path(), "acme").unwrap();
        assert_eq!(org.llm.provider, LlmProviderKind::Ollama);
    }

    #[test]
    fn server_config_reads_the_process_wide_sections() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &server_config_path(dir.path()),
            r#"
[bus]
kind = "nats"
nats_url = "nats://broker:4222"

[[mcp.servers]]
name = "github"
command = ["npx", "server-github"]

[notifications]
vapid_public_key = "pub"
"#,
        );
        let server = load_server_config(dir.path()).unwrap();
        assert_eq!(server.bus.kind, BusKind::Nats);
        assert_eq!(server.mcp.servers.len(), 1);
        assert_eq!(
            server.notifications.vapid_public_key.as_deref(),
            Some("pub")
        );
    }

    #[test]
    fn mcp_server_names_that_integration_tools_use_are_rejected_with_a_clear_error() {
        for name in ["org_linear", "space_sentry", "user_github"] {
            let dir = tempfile::tempdir().unwrap();
            write(
                &server_config_path(dir.path()),
                &format!("[[mcp.servers]]\nname = \"{name}\"\ncommand = [\"x\"]\n"),
            );
            let err = format!("{:#}", load_server_config(dir.path()).unwrap_err());
            assert!(err.contains(name), "names the server: {err}");
            assert!(err.contains("reserved"), "says why: {err}");
            assert!(err.contains("server.toml"), "names the file: {err}");
        }
    }

    #[test]
    fn ordinary_mcp_server_names_still_load() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &server_config_path(dir.path()),
            "[[mcp.servers]]\nname = \"user-notes\"\ncommand = [\"x\"]\n",
        );
        assert_eq!(load_server_config(dir.path()).unwrap().mcp.servers.len(), 1);
    }

    #[test]
    fn org_config_reads_the_tenant_sections() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &org_config_path(dir.path(), "acme").unwrap(),
            r#"
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[agent]
id = "ops"

[slack]
bot_token = "xoxb-1"
app_token = "xapp-1"
"#,
        );
        let org = load_org_config(dir.path(), "acme").unwrap();
        assert_eq!(org.llm.provider, LlmProviderKind::Anthropic);
        assert_eq!(org.llm.model, "claude-sonnet-4-20250514");
        assert_eq!(org.agent.id, "ops");
        assert!(org.slack.is_some());
    }

    #[test]
    fn each_org_gets_its_own_config() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &org_config_path(dir.path(), "acme").unwrap(),
            "[llm]\nprovider = \"openai\"\n",
        );
        let acme = load_org_config(dir.path(), "acme").unwrap();
        let other = load_org_config(dir.path(), "other").unwrap();
        assert_eq!(acme.llm.provider, LlmProviderKind::OpenAI);
        assert_eq!(other.llm.provider, LlmProviderKind::Ollama);
    }

    #[test]
    fn a_malformed_file_is_an_error_naming_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = org_config_path(dir.path(), "acme").unwrap();
        write(&path, "{{{{not valid toml");
        let err = load_org_config(dir.path(), "acme").unwrap_err();
        assert!(
            format!("{err:#}").contains(&path.display().to_string()),
            "error should name {}: {err:#}",
            path.display()
        );
        write(&server_config_path(dir.path()), "{{{{not valid toml");
        assert!(load_server_config(dir.path()).is_err());
    }

    /// `config.toml` is no longer read.
    #[test]
    fn a_leftover_config_toml_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("config.toml"),
            "[llm]\nprovider = \"anthropic\"\n",
        );
        let org = load_org_config(dir.path(), "default").unwrap();
        assert_eq!(org.llm.provider, LlmProviderKind::Ollama);
    }
}
