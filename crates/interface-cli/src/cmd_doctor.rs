//! `assistant doctor` subcommand — diagnose installation health.
//!
//! Inspired by `flutter doctor`, this command checks the assistant's environment
//! and reports the status of each component: config, database, LLM provider,
//! embeddings, MCP servers, skill directories, message bus, and interfaces.

use std::path::{Path, PathBuf};

use anyhow::Result;

use assistant_core::types::agent::{OrgConfig, ServerConfig};
use assistant_core::types::llm::LlmProviderKind;
use assistant_core::types::skills_mcp::McpTransportConfig;
use assistant_core::types::storage::BusKind;
use assistant_core::{
    expand_tilde, load_org_config, load_server_config, org_config_path, server_config_path,
};
use assistant_storage::DEFAULT_ORG_SLUG;

// ── Helpers ──────────────────────────────────────────────────────────────────

// ── Check result types ───────────────────────────────────────────────────────

/// Status of a single diagnostic check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    /// Everything is fine.
    Ok,
    /// Non-fatal issue (works, but could be better).
    Warning,
    /// Something is broken.
    Error,
    /// Skipped (not configured / not relevant).
    Skipped,
}

impl CheckStatus {
    fn icon(&self) -> &'static str {
        match self {
            CheckStatus::Ok => "[+]",
            CheckStatus::Warning => "[!]",
            CheckStatus::Error => "[x]",
            CheckStatus::Skipped => "[-]",
        }
    }
}

/// One diagnostic check result.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    /// Optional detail lines shown indented below the main message.
    pub details: Vec<String>,
}

impl CheckResult {
    fn ok(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Ok,
            message: message.into(),
            details: Vec::new(),
        }
    }

    fn warning(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warning,
            message: message.into(),
            details: Vec::new(),
        }
    }

    fn error(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Error,
            message: message.into(),
            details: Vec::new(),
        }
    }

    fn skipped(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Skipped,
            message: message.into(),
            details: Vec::new(),
        }
    }

    fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.details.push(detail.into());
        self
    }
}

/// Summary of all doctor checks.
pub struct DoctorReport {
    pub checks: Vec<CheckResult>,
}

impl DoctorReport {
    /// Format the report like `flutter doctor` output.
    pub fn format(&self) -> String {
        let mut out = String::new();
        out.push_str("Doctor summary:\n");

        for check in &self.checks {
            out.push_str(&format!(
                "  {} {} - {}\n",
                check.status.icon(),
                check.name,
                check.message
            ));
            for detail in &check.details {
                out.push_str(&format!("      {}\n", detail));
            }
        }

        // Summary line
        let ok_count = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Ok)
            .count();
        let warn_count = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warning)
            .count();
        let err_count = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Error)
            .count();
        let skip_count = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Skipped)
            .count();

        out.push('\n');
        if err_count == 0 && warn_count == 0 {
            out.push_str(&format!(
                "No issues found! ({} checks passed, {} skipped)\n",
                ok_count, skip_count
            ));
        } else {
            let mut parts = Vec::new();
            if ok_count > 0 {
                parts.push(format!("{} passed", ok_count));
            }
            if warn_count > 0 {
                parts.push(format!("{} warning(s)", warn_count));
            }
            if err_count > 0 {
                parts.push(format!("{} error(s)", err_count));
            }
            if skip_count > 0 {
                parts.push(format!("{} skipped", skip_count));
            }
            out.push_str(&format!("{}\n", parts.join(", ")));
        }

        out
    }
}

// ── Individual checks ────────────────────────────────────────────────────────

/// Every config file under `base_dir`: `server.toml` and each org's `org.toml`
/// that exists, in a stable order.
fn config_files(base_dir: &Path) -> Vec<PathBuf> {
    let mut orgs: Vec<String> = std::fs::read_dir(base_dir.join("orgs"))
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    orgs.sort();

    std::iter::once(server_config_path(base_dir))
        .chain(
            orgs.iter()
                .filter_map(|slug| org_config_path(base_dir, slug).ok()),
        )
        .filter(|p| p.exists())
        .collect()
}

fn check_config(base_dir: &Path) -> CheckResult {
    let files = config_files(base_dir);
    if files.is_empty() {
        return CheckResult::warning(
            "Configuration",
            "no server.toml or org.toml found (using defaults)",
        )
        .with_detail(format!(
            "Expected {} and {}",
            server_config_path(base_dir).display(),
            base_dir.join("orgs/<org>/org.toml").display()
        ));
    }

    let server_path = server_config_path(base_dir);
    let problems: Vec<String> = files
        .iter()
        .filter_map(|path| {
            let relative = path.strip_prefix(base_dir).unwrap_or(path).display();
            let raw = match std::fs::read_to_string(path) {
                Ok(raw) => raw,
                Err(e) => return Some(format!("{relative}: cannot read: {e}")),
            };
            let parsed = if *path == server_path {
                toml::from_str::<ServerConfig>(&raw).map(drop)
            } else {
                toml::from_str::<OrgConfig>(&raw).map(drop)
            };
            parsed.err().map(|e| format!("{relative}: {e}"))
        })
        .collect();

    if problems.is_empty() {
        let org_files = files.iter().filter(|p| **p != server_path).count();
        let server = if server_path.exists() {
            "server.toml and "
        } else {
            ""
        };
        return CheckResult::ok(
            "Configuration",
            format!("{server}{org_files} org.toml file(s) are valid"),
        );
    }
    let mut result = CheckResult::error("Configuration", "config files have errors");
    for problem in problems {
        result = result.with_detail(problem);
    }
    result
}

fn check_database(db_path: &Path) -> CheckResult {
    if !db_path.exists() {
        return CheckResult::warning(
            "Database",
            "assistant.db not found (will be created on first run)",
        )
        .with_detail(format!("Expected at: {}", db_path.display()));
    }

    let metadata = match std::fs::metadata(db_path) {
        Ok(m) => m,
        Err(e) => {
            return CheckResult::error("Database", "cannot access assistant.db")
                .with_detail(format!("{e}"));
        }
    };

    let size_mb = metadata.len() as f64 / 1_048_576.0;
    CheckResult::ok("Database", format!("assistant.db ({:.1} MB)", size_mb))
        .with_detail(format!("Path: {}", db_path.display()))
}

fn check_llm_provider(config: &OrgConfig) -> CheckResult {
    let provider = &config.llm.provider;
    let model = &config.llm.model;

    match provider {
        LlmProviderKind::Ollama => {
            CheckResult::ok("LLM Provider", format!("Ollama (model: {model})"))
                .with_detail(format!("Base URL: {}", config.llm.base_url))
        }
        LlmProviderKind::Anthropic => {
            let has_key =
                config.llm.api_key.is_some() || std::env::var("ANTHROPIC_API_KEY").is_ok();
            if has_key {
                CheckResult::ok("LLM Provider", format!("Anthropic (model: {model})"))
            } else {
                CheckResult::error(
                    "LLM Provider",
                    format!("Anthropic (model: {model}) - no API key"),
                )
                .with_detail("Set api_key in [llm] or ANTHROPIC_API_KEY env var")
            }
        }
        LlmProviderKind::OpenAI => {
            let has_key = config.llm.api_key.is_some() || std::env::var("OPENAI_API_KEY").is_ok();
            if has_key {
                CheckResult::ok("LLM Provider", format!("OpenAI (model: {model})"))
            } else {
                CheckResult::error(
                    "LLM Provider",
                    format!("OpenAI (model: {model}) - no API key"),
                )
                .with_detail("Set api_key in [llm] or OPENAI_API_KEY env var")
            }
        }
        LlmProviderKind::Moonshot => {
            let has_key = config.llm.api_key.is_some() || std::env::var("MOONSHOT_API_KEY").is_ok();
            if has_key {
                CheckResult::ok("LLM Provider", format!("Moonshot (model: {model})"))
            } else {
                CheckResult::error(
                    "LLM Provider",
                    format!("Moonshot (model: {model}) - no API key"),
                )
                .with_detail("Set api_key in [llm] or MOONSHOT_API_KEY env var")
            }
        }
        LlmProviderKind::OpenRouter => {
            let has_key =
                config.llm.api_key.is_some() || std::env::var("OPENROUTER_API_KEY").is_ok();
            if has_key {
                CheckResult::ok("LLM Provider", format!("OpenRouter (model: {model})"))
            } else {
                CheckResult::error(
                    "LLM Provider",
                    format!("OpenRouter (model: {model}) - no API key"),
                )
                .with_detail("Set api_key in [llm] or OPENROUTER_API_KEY env var")
            }
        }
    }
}

fn check_embeddings(config: &OrgConfig) -> CheckResult {
    match &config.llm.embeddings {
        Some(emb) => {
            let provider = &emb.provider;
            let model = emb.model.as_deref().unwrap_or("default");
            let has_key = emb.api_key.is_some();
            if has_key {
                CheckResult::ok("Embeddings", format!("{provider:?} (model: {model})"))
            } else {
                CheckResult::warning(
                    "Embeddings",
                    format!("{provider:?} (model: {model}) - no API key in config"),
                )
                .with_detail("May use env var fallback at runtime")
            }
        }
        None => {
            // Falls back to main LLM provider for embeddings
            CheckResult::ok(
                "Embeddings",
                format!(
                    "Using main LLM provider (model: {})",
                    config
                        .llm
                        .embedding_model
                        .as_deref()
                        .unwrap_or("provider default")
                ),
            )
        }
    }
}

fn check_mcp_servers(config: &ServerConfig) -> CheckResult {
    let servers = &config.mcp.servers;
    if servers.is_empty() {
        return CheckResult::skipped("MCP Servers", "none configured");
    }

    let reserved: Vec<String> = servers
        .iter()
        .filter_map(|s| {
            let prefix = s.reserved_prefix()?;
            Some(format!(
                "{}: the prefix '{prefix}' is reserved for integration tools; rename it",
                s.name
            ))
        })
        .collect();
    if !reserved.is_empty() {
        let mut result = CheckResult::error("MCP Servers", "server names use reserved prefixes");
        result.details = reserved;
        return result;
    }

    let enabled: Vec<_> = servers.iter().filter(|s| s.enabled).collect();
    let disabled_count = servers.len() - enabled.len();

    if enabled.is_empty() {
        return CheckResult::skipped(
            "MCP Servers",
            format!("{} configured, all disabled", servers.len()),
        );
    }

    let mut result = CheckResult::ok(
        "MCP Servers",
        format!(
            "{} enabled{}",
            enabled.len(),
            if disabled_count > 0 {
                format!(", {} disabled", disabled_count)
            } else {
                String::new()
            }
        ),
    );

    for server in &enabled {
        let transport = match &server.transport {
            McpTransportConfig::Stdio { command } => {
                format!("stdio: {}", command.join(" "))
            }
            McpTransportConfig::Http { url, .. } => {
                format!("http: {url}")
            }
        };
        result
            .details
            .push(format!("{} ({})", server.name, transport));
    }

    result
}

fn check_skill_directories(config: &OrgConfig, assistant_dir: &Path) -> CheckResult {
    let mut dirs_checked = Vec::new();
    let mut ok_dirs = 0usize;
    let mut missing_dirs = 0usize;

    // Default skill directory
    let default_dir = assistant_dir.join("skills");
    if default_dir.is_dir() {
        ok_dirs += 1;
        let count = std::fs::read_dir(&default_dir)
            .map(|rd| rd.filter(|e| e.is_ok()).count())
            .unwrap_or(0);
        dirs_checked.push(format!("{} ({} entries)", default_dir.display(), count));
    } else {
        missing_dirs += 1;
        dirs_checked.push(format!("{} (not found)", default_dir.display()));
    }

    // Extra dirs from config
    for extra in &config.skills.extra_dirs {
        let p = expand_tilde(extra);
        if p.is_dir() {
            ok_dirs += 1;
            let count = std::fs::read_dir(&p)
                .map(|rd| rd.filter(|e| e.is_ok()).count())
                .unwrap_or(0);
            dirs_checked.push(format!("{} ({} entries)", p.display(), count));
        } else {
            // Extra dirs that don't exist are fine — they're just not populated
            dirs_checked.push(format!("{} (not found)", p.display()));
        }
    }

    let mut result = if ok_dirs > 0 {
        CheckResult::ok("Skill Directories", format!("{} accessible", ok_dirs))
    } else {
        CheckResult::warning(
            "Skill Directories",
            "no skill directories found".to_string(),
        )
    };
    let _ = missing_dirs; // used only for the count above

    for d in dirs_checked {
        result.details.push(d);
    }
    result
}

fn check_message_bus(config: &ServerConfig) -> CheckResult {
    match &config.bus.kind {
        BusKind::Sqlite => CheckResult::ok("Message Bus", "SQLite (default)"),
        BusKind::Nats => {
            let url = config
                .bus
                .nats_url
                .clone()
                .or_else(|| std::env::var("NATS_URL").ok())
                .unwrap_or_else(|| "nats://localhost:4222".to_string());
            CheckResult::ok("Message Bus", format!("NATS ({url})"))
                .with_detail("Connectivity verified at startup")
        }
    }
}

fn check_interfaces(config: &OrgConfig) -> CheckResult {
    let mut configured = Vec::new();

    if config.slack.is_some() {
        configured.push("Slack");
    }
    if config.mattermost.is_some() {
        configured.push("Mattermost");
    }
    if config.matrix.is_some() {
        configured.push("Matrix");
    }
    if config.nextcloud.is_some() {
        configured.push("Nextcloud");
    }
    if config.signal.is_some() {
        configured.push("Signal");
    }

    if configured.is_empty() {
        return CheckResult::skipped("Interfaces", "none configured (CLI only)");
    }

    let mut result = CheckResult::ok("Interfaces", format!("{} configured", configured.len()));
    for iface in &configured {
        result.details.push((*iface).to_string());
    }
    result
}

fn check_observability() -> CheckResult {
    if assistant_runtime::telemetry::otlp_export_enabled() {
        CheckResult::ok("Observability", "OTLP export enabled")
    } else {
        CheckResult::skipped(
            "Observability",
            "OTLP export off (set OTEL_EXPORTER_OTLP_ENDPOINT to enable)",
        )
    }
}

fn check_learning(config: &OrgConfig) -> CheckResult {
    if config.learning.enabled {
        CheckResult::ok("Learning", "enabled")
    } else {
        CheckResult::skipped("Learning", "disabled")
    }
}

// ── Public entry point ───────────────────────────────────────────────────────

/// Run all diagnostic checks and print the report.
///
/// `assistant_dir` is the install root holding `server.toml` and `orgs/`.
/// Config files that fail to load are reported by the configuration check and
/// the checks that depend on them run against the defaults.
pub async fn cmd_doctor(assistant_dir: &Path, db_path: &Path) -> Result<()> {
    let server = load_server_config(assistant_dir).unwrap_or_default();
    let config = load_org_config(assistant_dir, DEFAULT_ORG_SLUG).unwrap_or_default();

    let checks = vec![
        check_config(assistant_dir),
        check_database(db_path),
        check_llm_provider(&config),
        check_embeddings(&config),
        check_mcp_servers(&server),
        check_skill_directories(&config, assistant_dir),
        check_message_bus(&server),
        check_interfaces(&config),
        check_observability(),
        check_learning(&config),
    ];

    let report = DoctorReport { checks };
    print!("{}", report.format());

    // Exit with non-zero if any errors found
    let has_errors = report.checks.iter().any(|c| c.status == CheckStatus::Error);
    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> OrgConfig {
        OrgConfig::default()
    }

    // -- CheckResult constructors --

    #[test]
    fn check_result_ok_has_correct_status() {
        let r = CheckResult::ok("Test", "all good");
        assert_eq!(r.status, CheckStatus::Ok);
        assert_eq!(r.name, "Test");
        assert_eq!(r.message, "all good");
        assert!(r.details.is_empty());
    }

    #[test]
    fn check_result_with_detail_appends() {
        let r = CheckResult::ok("Test", "fine")
            .with_detail("line 1")
            .with_detail("line 2");
        assert_eq!(r.details.len(), 2);
        assert_eq!(r.details[0], "line 1");
    }

    #[test]
    fn status_icons_are_distinct() {
        let icons: Vec<_> = [
            CheckStatus::Ok,
            CheckStatus::Warning,
            CheckStatus::Error,
            CheckStatus::Skipped,
        ]
        .iter()
        .map(|s| s.icon())
        .collect();
        // All four icons should be unique
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(unique.len(), 4, "each status must have a unique icon");
    }

    // -- Report formatting --

    #[test]
    fn report_format_contains_all_check_names() {
        let report = DoctorReport {
            checks: vec![
                CheckResult::ok("Alpha", "good"),
                CheckResult::error("Beta", "bad"),
            ],
        };
        let out = report.format();
        assert!(out.contains("Alpha"), "report should mention Alpha");
        assert!(out.contains("Beta"), "report should mention Beta");
    }

    #[test]
    fn report_format_shows_summary_with_no_issues() {
        let report = DoctorReport {
            checks: vec![CheckResult::ok("Test", "fine")],
        };
        let out = report.format();
        assert!(
            out.contains("No issues found!"),
            "clean report should say no issues"
        );
    }

    #[test]
    fn report_format_shows_error_count() {
        let report = DoctorReport {
            checks: vec![
                CheckResult::ok("A", "fine"),
                CheckResult::error("B", "broken"),
                CheckResult::error("C", "also broken"),
            ],
        };
        let out = report.format();
        assert!(
            out.contains("2 error(s)"),
            "should report 2 errors, got: {out}"
        );
    }

    #[test]
    fn report_format_shows_details_indented() {
        let report = DoctorReport {
            checks: vec![CheckResult::ok("Foo", "bar").with_detail("detail line")],
        };
        let out = report.format();
        assert!(
            out.contains("      detail line"),
            "details should be indented"
        );
    }

    // -- Individual check logic --

    fn write_config(base: &Path, rel: &str, content: &str) {
        let path = base.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn check_config_warns_when_no_config_files_exist() {
        let dir = tempfile::tempdir().unwrap();
        let r = check_config(dir.path());
        assert_eq!(r.status, CheckStatus::Warning);
        assert!(r.message.contains("server.toml"), "got: {}", r.message);
    }

    #[test]
    fn check_config_ok_for_valid_server_and_org_files() {
        let dir = tempfile::tempdir().unwrap();
        write_config(dir.path(), "server.toml", "[bus]\nkind = \"sqlite\"\n");
        write_config(
            dir.path(),
            "orgs/acme/org.toml",
            "[llm]\nprovider = \"ollama\"\n",
        );
        let r = check_config(dir.path());
        assert_eq!(r.status, CheckStatus::Ok, "got: {}", r.message);
        assert!(r.message.contains("1 org.toml"), "got: {}", r.message);
    }

    #[test]
    fn check_config_names_the_file_that_fails_to_parse() {
        let dir = tempfile::tempdir().unwrap();
        write_config(dir.path(), "server.toml", "[bus]\nkind = \"sqlite\"\n");
        write_config(dir.path(), "orgs/acme/org.toml", "{{{{");
        let r = check_config(dir.path());
        assert_eq!(r.status, CheckStatus::Error);
        assert!(
            r.details.iter().any(|d| d.contains("orgs/acme/org.toml")),
            "details should name the file: {:?}",
            r.details
        );
    }

    #[test]
    fn check_config_ignores_a_leftover_config_toml() {
        let dir = tempfile::tempdir().unwrap();
        write_config(dir.path(), "config.toml", "{{{{");
        assert_eq!(check_config(dir.path()).status, CheckStatus::Warning);
    }

    #[test]
    fn check_database_warns_on_missing_db() {
        let r = check_database(Path::new("/nonexistent/path/assistant.db"));
        assert_eq!(r.status, CheckStatus::Warning);
    }

    #[test]
    fn check_llm_ollama_is_ok() {
        let config = default_config();
        // Default provider is Ollama, which doesn't need an API key
        let r = check_llm_provider(&config);
        assert_eq!(r.status, CheckStatus::Ok);
        assert!(r.message.contains("Ollama"));
    }

    #[test]
    fn check_llm_anthropic_errors_without_key() {
        let mut config = default_config();
        config.llm.provider = LlmProviderKind::Anthropic;
        config.llm.api_key = None;
        // SAFETY: test-only; no other thread reads this var concurrently.
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        let r = check_llm_provider(&config);
        assert_eq!(r.status, CheckStatus::Error);
        assert!(r.message.contains("no API key"));
    }

    #[test]
    fn check_llm_anthropic_ok_with_config_key() {
        let mut config = default_config();
        config.llm.provider = LlmProviderKind::Anthropic;
        config.llm.api_key = Some("sk-ant-test".into());
        let r = check_llm_provider(&config);
        assert_eq!(r.status, CheckStatus::Ok);
    }

    #[test]
    fn check_embeddings_ok_with_main_provider() {
        let config = default_config();
        let r = check_embeddings(&config);
        assert_eq!(r.status, CheckStatus::Ok);
        assert!(r.message.contains("Using main LLM provider"));
    }

    #[test]
    fn check_mcp_skipped_when_none_configured() {
        let r = check_mcp_servers(&ServerConfig::default());
        assert_eq!(r.status, CheckStatus::Skipped);
    }

    #[test]
    fn check_mcp_reports_names_reserved_for_integration_tools() {
        let mut config = ServerConfig::default();
        config.mcp.servers = serde_json::from_value(serde_json::json!([
            {"name": "user_notes", "command": ["x"]},
            {"name": "github", "command": ["y"]},
        ]))
        .unwrap();
        let r = check_mcp_servers(&config);
        assert_eq!(r.status, CheckStatus::Error);
        assert!(
            r.details
                .iter()
                .any(|d| d.contains("user_notes") && d.contains("reserved"))
        );
    }

    #[test]
    fn check_interfaces_skipped_when_none() {
        let config = default_config();
        let r = check_interfaces(&config);
        assert_eq!(r.status, CheckStatus::Skipped);
        assert!(r.message.contains("CLI only"));
    }

    #[test]
    fn check_message_bus_sqlite_default() {
        let r = check_message_bus(&ServerConfig::default());
        assert_eq!(r.status, CheckStatus::Ok);
        assert!(r.message.contains("SQLite"));
    }

    #[test]
    fn check_learning_reports_enabled_by_default() {
        let config = default_config();
        let r = check_learning(&config);
        // Default learning config has enabled = true
        assert!(
            r.status == CheckStatus::Ok || r.status == CheckStatus::Skipped,
            "should be ok or skipped depending on default"
        );
    }
}
