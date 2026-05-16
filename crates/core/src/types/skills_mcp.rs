//! Configuration for skill discovery, external MCP server connections, and
//! the legacy mirror/trace section.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::default_true;

// ── Skills ────────────────────────────────────────────────────────────────────

/// Skills configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// Extra directories to scan for Agent Skills.
    /// Defaults cover Claude Code / NanoClaw shared skill folders.
    #[serde(default = "default_skill_extra_dirs")]
    pub extra_dirs: Vec<String>,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            extra_dirs: default_skill_extra_dirs(),
        }
    }
}

fn default_skill_extra_dirs() -> Vec<String> {
    vec![
        "~/.claude/skills".to_string(),
        "./.claude/skills".to_string(),
    ]
}

// ── MCP ───────────────────────────────────────────────────────────────────────

/// MCP configuration — covers external MCP client connections.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// External MCP servers to connect to as a client.
    /// Each entry spawns a connection at startup and bridges the server's tools
    /// into the assistant's tool registry.
    #[serde(default)]
    pub servers: Vec<McpServerEntry>,
}

/// An external MCP server the assistant connects to as a client.
///
/// Tools discovered from the server are registered with the prefix
/// `mcp__{name}__{tool_name}` to avoid collisions with builtin tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Unique name for this server (used in tool prefix).
    /// Must be a valid identifier: lowercase alphanumeric + hyphens.
    pub name: String,
    /// Transport configuration — determines how we connect to the server.
    #[serde(flatten)]
    pub transport: McpTransportConfig,
    /// Extra environment variables passed to stdio-spawned servers.
    /// Values may reference environment variables with `${VAR}` syntax.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether this server is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Trust level controlling safety-gate behaviour for this server's tools.
    #[serde(default)]
    pub trust: McpTrustLevel,
}

/// How to connect to an external MCP server.
///
/// Uses `#[serde(untagged)]` for compatibility with the standard MCP
/// configuration format (Claude Desktop, VS Code, etc.).  Serde tries
/// variants in order: if `command` is present the entry is treated as
/// `Stdio`; otherwise `url` selects `Http`.  If a config entry contains
/// *both* `command` and `url`, `Stdio` wins and `url` is silently
/// ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpTransportConfig {
    /// Spawn a local subprocess; communicate via JSON-RPC over stdin/stdout.
    Stdio {
        /// Command and its arguments (e.g. `["npx", "-y", "@mcp/server-github"]`).
        command: Vec<String>,
    },
    /// Connect to a remote HTTP endpoint (SSE or Streamable HTTP).
    Http {
        /// Server URL (e.g. `"https://example.com/mcp/sse"`).
        url: String,
        /// Extra HTTP headers sent with every request.
        /// Values may reference environment variables with `${VAR}` syntax.
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// Trust level for tools from an external MCP server.
///
/// Controls whether the safety gate requires user confirmation before executing
/// a tool call from this server.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum McpTrustLevel {
    /// Every tool call requires explicit user confirmation (default).
    #[default]
    Confirm,
    /// Tools may run without confirmation (use for trusted, read-only servers).
    Trust,
}

// ── Mirror (legacy trace config) ──────────────────────────────────────────────

/// Self-improvement / tracing config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    pub trace_enabled: bool,
    /// When `true`, LLM span events include full message content
    /// (`gen_ai.input.messages`, `gen_ai.output.messages`, etc.).
    /// Off by default because content may contain PII.
    #[serde(default)]
    pub trace_content: bool,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            trace_enabled: true,
            trace_content: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{McpConfig, McpTransportConfig, McpTrustLevel};

    #[test]
    fn mcp_config_default_has_empty_servers() {
        let cfg = McpConfig::default();
        assert!(cfg.servers.is_empty());
    }

    #[test]
    fn mcp_config_stdio_server() {
        let toml_str = r#"
            [[servers]]
            name = "github"
            command = ["npx", "-y", "@modelcontextprotocol/server-github"]

            [servers.env]
            GITHUB_TOKEN = "gh-token-123"
        "#;
        let cfg: McpConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.servers.len(), 1);
        let s = &cfg.servers[0];
        assert_eq!(s.name, "github");
        assert!(s.enabled);
        assert_eq!(s.trust, McpTrustLevel::Confirm);
        match &s.transport {
            McpTransportConfig::Stdio { command } => {
                assert_eq!(
                    command,
                    &["npx", "-y", "@modelcontextprotocol/server-github"]
                );
            }
            other => panic!("expected Stdio, got {other:?}"),
        }
        assert_eq!(s.env.get("GITHUB_TOKEN").unwrap(), "gh-token-123");
    }

    #[test]
    fn mcp_config_http_server() {
        let toml_str = r#"
            [[servers]]
            name = "remote-db"
            url = "https://db.example.com/mcp/sse"
            trust = "trust"

            [servers.headers]
            Authorization = "Bearer secret"
        "#;
        let cfg: McpConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.servers.len(), 1);
        let s = &cfg.servers[0];
        assert_eq!(s.name, "remote-db");
        assert_eq!(s.trust, McpTrustLevel::Trust);
        match &s.transport {
            McpTransportConfig::Http { url, headers } => {
                assert_eq!(url, "https://db.example.com/mcp/sse");
                assert_eq!(headers.get("Authorization").unwrap(), "Bearer secret");
            }
            other => panic!("expected Http, got {other:?}"),
        }
    }

    #[test]
    fn mcp_config_multiple_servers() {
        let toml_str = r#"
            [[servers]]
            name = "fs"
            command = ["mcp-server-fs", "/tmp"]

            [[servers]]
            name = "api"
            url = "https://api.example.com/mcp"
            enabled = false
        "#;
        let cfg: McpConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.servers.len(), 2);
        assert!(cfg.servers[0].enabled);
        assert!(!cfg.servers[1].enabled);
    }

    #[test]
    fn mcp_config_no_servers_section_defaults_empty() {
        let toml_str = r#""#;
        let cfg: McpConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.servers.is_empty());
    }

    #[test]
    fn mcp_trust_level_default_is_confirm() {
        let level = McpTrustLevel::default();
        assert_eq!(level, McpTrustLevel::Confirm);
    }
}
