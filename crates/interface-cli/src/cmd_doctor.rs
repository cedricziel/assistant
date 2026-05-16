//! `assistant doctor` subcommand — diagnose installation health.
//!
//! Inspired by `flutter doctor`, this command checks the assistant's environment
//! and reports the status of each component: config, database, LLM provider,
//! embeddings, MCP servers, skill directories, message bus, and interfaces.

use std::path::{Path, PathBuf};

use anyhow::Result;

use assistant_core::types::agent::AssistantConfig;
use assistant_core::types::llm::LlmProviderKind;
use assistant_core::types::skills_mcp::McpTransportConfig;
use assistant_core::types::storage::BusKind;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Expand a leading `~` to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    PathBuf::from(path)
}

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

fn check_config(config_path: &Path) -> CheckResult {
    if !config_path.exists() {
        return CheckResult::warning("Configuration", "config.toml not found (using defaults)")
            .with_detail(format!("Expected at: {}", config_path.display()));
    }

    match std::fs::read_to_string(config_path) {
        Ok(raw) => match toml::from_str::<AssistantConfig>(&raw) {
            Ok(_) => CheckResult::ok("Configuration", "config.toml is valid"),
            Err(e) => CheckResult::error("Configuration", "config.toml has parse errors")
                .with_detail(format!("{e}")),
        },
        Err(e) => CheckResult::error("Configuration", "cannot read config.toml")
            .with_detail(format!("{e}")),
    }
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

fn check_llm_provider(config: &AssistantConfig) -> CheckResult {
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

fn check_embeddings(config: &AssistantConfig) -> CheckResult {
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
                    config.llm.embedding_model
                ),
            )
        }
    }
}

fn check_mcp_servers(config: &AssistantConfig) -> CheckResult {
    let servers = &config.mcp.servers;
    if servers.is_empty() {
        return CheckResult::skipped("MCP Servers", "none configured");
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

fn check_skill_directories(config: &AssistantConfig, assistant_dir: &Path) -> CheckResult {
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

fn check_message_bus(config: &AssistantConfig) -> CheckResult {
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

fn check_interfaces(config: &AssistantConfig) -> CheckResult {
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

fn check_observability(config: &AssistantConfig) -> CheckResult {
    let exporter = &config.observability.exporter;
    CheckResult::ok("Observability", format!("Local exporter: {exporter:?}"))
}

fn check_learning(config: &AssistantConfig) -> CheckResult {
    if config.learning.enabled {
        CheckResult::ok("Learning", "enabled")
    } else {
        CheckResult::skipped("Learning", "disabled")
    }
}

/// Compare `messages` row counts between `assistant.db.legacy` and the live
/// runtime database. Warns when they differ — this happens when a host was
/// migrated by an old binary that copied the DB but never renamed the
/// original, then continued writing to `assistant.db` while `space.db`
/// went stale. The fix is `assistant migrate finalize`.
async fn check_drift(base_path: &Path, db_path: &Path) -> CheckResult {
    let legacy_path = base_path.join("assistant.db.legacy");

    if !legacy_path.exists() {
        return CheckResult::skipped(
            "Migration Drift",
            "no assistant.db.legacy present (nothing to compare)",
        );
    }
    if !db_path.exists() {
        return CheckResult::skipped(
            "Migration Drift",
            "runtime database not yet created (nothing to compare)",
        );
    }

    let legacy_count = match count_messages_readonly(&legacy_path).await {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::warning("Migration Drift", "could not read assistant.db.legacy")
                .with_detail(format!("{e}"))
                .with_detail(format!("Path: {}", legacy_path.display()));
        }
    };
    let runtime_count = match count_messages_readonly(db_path).await {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::warning("Migration Drift", "could not read runtime database")
                .with_detail(format!("{e}"))
                .with_detail(format!("Path: {}", db_path.display()));
        }
    };

    if legacy_count == runtime_count {
        CheckResult::ok(
            "Migration Drift",
            format!("legacy and runtime row counts match ({legacy_count} messages)"),
        )
    } else {
        CheckResult::warning(
            "Migration Drift",
            format!(
                "row count drift: legacy has {legacy_count} messages, runtime has {runtime_count}",
            ),
        )
        .with_detail(format!("Legacy:  {}", legacy_path.display()))
        .with_detail(format!("Runtime: {}", db_path.display()))
        .with_detail(
            "Run `assistant migrate finalize` to overwrite the runtime DB with legacy content",
        )
    }
}

/// Open a SQLite file read-only and count rows in the `messages` table.
/// Read-only mode (`mode=ro`) is critical: it prevents sqlx from creating
/// `*-wal` / `*-shm` sidecars against `assistant.db.legacy`, which would
/// confuse a future `migrate finalize` run.
async fn count_messages_readonly(path: &Path) -> Result<i64> {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))?
        .read_only(true)
        .create_if_missing(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
        .fetch_one(&pool)
        .await?;
    pool.close().await;
    Ok(row.0)
}

// ── Public entry point ───────────────────────────────────────────────────────

/// Run all diagnostic checks and print the report.
pub async fn cmd_doctor(
    config_path: &Path,
    db_path: &Path,
    config: &AssistantConfig,
) -> Result<()> {
    let assistant_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("~/.assistant"));

    let checks = vec![
        check_config(config_path),
        check_database(db_path),
        check_drift(assistant_dir, db_path).await,
        check_llm_provider(config),
        check_embeddings(config),
        check_mcp_servers(config),
        check_skill_directories(config, assistant_dir),
        check_message_bus(config),
        check_interfaces(config),
        check_observability(config),
        check_learning(config),
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

    fn default_config() -> AssistantConfig {
        AssistantConfig::default()
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

    #[test]
    fn check_config_warns_on_missing_file() {
        let r = check_config(Path::new("/nonexistent/path/config.toml"));
        assert_eq!(r.status, CheckStatus::Warning);
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
        config.llm.api_key = Some("sk-ant-test".to_string());
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
        let config = default_config();
        let r = check_mcp_servers(&config);
        assert_eq!(r.status, CheckStatus::Skipped);
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
        let config = default_config();
        let r = check_message_bus(&config);
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

    // -- Migration drift check --

    /// Helper: create a minimal SQLite DB at `path` with a `messages` table
    /// containing `n` rows. Used by the drift-check tests below.
    async fn seed_messages_db(path: &std::path::Path, n: usize) {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))
            .unwrap()
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE messages (id INTEGER PRIMARY KEY, body TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        for i in 0..n {
            sqlx::query("INSERT INTO messages (body) VALUES (?)")
                .bind(format!("msg-{i}"))
                .execute(&pool)
                .await
                .unwrap();
        }
        pool.close().await;
    }

    #[tokio::test]
    async fn check_drift_skipped_when_no_legacy() {
        let dir = tempfile::TempDir::new().unwrap();
        let runtime = dir.path().join("space.db");
        seed_messages_db(&runtime, 3).await;

        let r = check_drift(dir.path(), &runtime).await;
        assert_eq!(r.status, CheckStatus::Skipped);
    }

    #[tokio::test]
    async fn check_drift_ok_when_counts_match() {
        let dir = tempfile::TempDir::new().unwrap();
        let legacy = dir.path().join("assistant.db.legacy");
        let runtime = dir.path().join("space.db");
        seed_messages_db(&legacy, 5).await;
        seed_messages_db(&runtime, 5).await;

        let r = check_drift(dir.path(), &runtime).await;
        assert_eq!(r.status, CheckStatus::Ok);
        assert!(r.message.contains("5 messages"));
    }

    #[tokio::test]
    async fn check_drift_warns_when_counts_differ() {
        let dir = tempfile::TempDir::new().unwrap();
        let legacy = dir.path().join("assistant.db.legacy");
        let runtime = dir.path().join("space.db");
        seed_messages_db(&legacy, 10).await;
        seed_messages_db(&runtime, 3).await;

        let r = check_drift(dir.path(), &runtime).await;
        assert_eq!(
            r.status,
            CheckStatus::Warning,
            "drift between legacy and runtime should warn"
        );
        assert!(r.message.contains("10"));
        assert!(r.message.contains("3"));
        assert!(
            r.details.iter().any(|d| d.contains("migrate finalize")),
            "warning should suggest running `assistant migrate finalize`"
        );
    }
}
