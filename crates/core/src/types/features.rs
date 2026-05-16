//! Per-feature runtime configuration (memory, compaction, learning, titling,
//! push notifications).

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

// ── Memory ────────────────────────────────────────────────────────────────────

/// Configuration for the agent's persistent markdown memory files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Whether memory loading is enabled (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path to AGENTS.md — workspace rules, session startup ritual, memory discipline
    pub agents_path: Option<String>,
    /// Path to SOUL.md — personality, values, core truths
    pub soul_path: Option<String>,
    /// Path to IDENTITY.md — name, role, structured identity profile
    pub identity_path: Option<String>,
    /// Path to USER.md — user profile, preferences, timezone
    pub user_path: Option<String>,
    /// Path to TOOLS.md — environment-specific tool notes (SSH hosts, devices, etc.)
    pub tools_path: Option<String>,
    /// Path to MEMORY.md — curated long-term memory
    pub memory_path: Option<String>,
    /// Directory for daily append-only notes (YYYY-MM-DD.md)
    pub notes_dir: Option<String>,
    /// Path to BOOTSTRAP.md — first-run onboarding ritual (self-deleting)
    pub bootstrap_path: Option<String>,
    /// Path to HEARTBEAT.md — periodic task checklist for the scheduler
    pub heartbeat_path: Option<String>,
    /// Path to BOOT.md — per-session startup hook
    pub boot_path: Option<String>,
    /// How often to run the memory indexer (in seconds). Default: 300 (5 minutes).
    #[serde(default = "default_indexing_interval")]
    pub indexing_interval_seconds: Option<u64>,
}

fn default_indexing_interval() -> Option<u64> {
    Some(300)
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            agents_path: None,
            soul_path: None,
            identity_path: None,
            user_path: None,
            tools_path: None,
            memory_path: None,
            notes_dir: None,
            bootstrap_path: None,
            heartbeat_path: None,
            boot_path: None,
            indexing_interval_seconds: default_indexing_interval(),
        }
    }
}

// ── Compaction ────────────────────────────────────────────────────────────────

/// Context compaction configuration.
///
/// When the accumulated token count in a conversation exceeds
/// `context_window_tokens - reserve_floor_tokens - soft_threshold_tokens`,
/// the orchestrator triggers a silent compaction turn: it writes a memory
/// summary and drops old history, keeping only the most recent turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Whether context compaction is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Estimated total context window in tokens (default: 200_000).
    #[serde(default = "default_compaction_context_window")]
    pub context_window_tokens: u64,
    /// Tokens reserved for the assistant's output headroom (default: 20_000).
    #[serde(default = "default_compaction_reserve_floor")]
    pub reserve_floor_tokens: u64,
    /// Soft threshold: trigger compaction this many tokens before the hard
    /// limit (default: 4_000).
    #[serde(default = "default_compaction_soft_threshold")]
    pub soft_threshold_tokens: u64,
    /// How many of the most recent turns to keep verbatim after compaction
    /// (default: 10).
    #[serde(default = "default_keep_recent_turns")]
    pub keep_recent_turns: usize,
}

fn default_compaction_context_window() -> u64 {
    200_000
}

fn default_compaction_reserve_floor() -> u64 {
    20_000
}

fn default_compaction_soft_threshold() -> u64 {
    30_000
}

fn default_keep_recent_turns() -> usize {
    10
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            context_window_tokens: default_compaction_context_window(),
            reserve_floor_tokens: default_compaction_reserve_floor(),
            soft_threshold_tokens: default_compaction_soft_threshold(),
            keep_recent_turns: default_keep_recent_turns(),
        }
    }
}

// ── Learning ──────────────────────────────────────────────────────────────────

/// Autonomous learning configuration (`[learning]` section).
///
/// Controls skill-scoped tracing, autonomous skill creation from completed
/// tasks, and periodic self-improvement of existing skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Master switch for all learning features (default: false).
    #[serde(default)]
    pub enabled: bool,
    /// Automatically create new skills from novel, complex tasks (default: true).
    #[serde(default = "default_true")]
    pub auto_create_skills: bool,
    /// Automatically apply skill refinements without `/review` (default: true).
    #[serde(default = "default_true")]
    pub auto_apply_refinements: bool,
    /// Minimum tool calls in a turn before considering skill creation (default: 3).
    #[serde(default = "default_min_tool_calls")]
    pub min_tool_calls_for_skill: usize,
    /// Minimum traced executions before a skill is eligible for self-analysis (default: 10).
    #[serde(default = "default_min_executions")]
    pub min_executions_for_analysis: i64,
    /// Cron expression for the periodic improvement task (default: every 6 hours).
    #[serde(default = "default_improvement_cron")]
    pub improvement_cron: String,
    /// Error rate above which a skill is considered underperforming (default: 0.2).
    #[serde(default = "default_error_rate_threshold")]
    pub error_rate_threshold: f64,
    /// Number of executions after applying a refinement before evaluating regression (default: 5).
    #[serde(default = "default_revert_window")]
    pub revert_window: i64,
    /// Error rate increase (absolute) that triggers a revert (default: 0.1).
    #[serde(default = "default_revert_regression_threshold")]
    pub revert_regression_threshold: f64,
}

fn default_min_tool_calls() -> usize {
    3
}
fn default_min_executions() -> i64 {
    10
}
fn default_improvement_cron() -> String {
    "0 */6 * * *".to_string()
}
fn default_error_rate_threshold() -> f64 {
    0.2
}
fn default_revert_window() -> i64 {
    5
}
fn default_revert_regression_threshold() -> f64 {
    0.1
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_create_skills: true,
            auto_apply_refinements: true,
            min_tool_calls_for_skill: default_min_tool_calls(),
            min_executions_for_analysis: default_min_executions(),
            improvement_cron: default_improvement_cron(),
            error_rate_threshold: default_error_rate_threshold(),
            revert_window: default_revert_window(),
            revert_regression_threshold: default_revert_regression_threshold(),
        }
    }
}

// ── Titling ───────────────────────────────────────────────────────────────────

/// Conversation title-generator configuration (`[titling]` section).
///
/// Controls when the background title-generator worker assigns titles to
/// conversations.  The worker consumes `turn.result` from the message bus
/// and is shared across every interface, so this single block governs
/// titling for web, CLI, MCP, scheduler, and all messenger adapters.
///
/// The worker always uses the conversation's primary LLM provider for the
/// title call.  A future change can re-introduce a per-org model override
/// once the `LlmProvider` trait exposes a model-selection knob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitlingConfig {
    /// Master switch (default: `true`).  When `false`, the worker still
    /// consumes and acks `turn.result` messages but never calls the LLM.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Minimum turn number at which an otherwise-unlocked conversation
    /// becomes eligible for an auto-title (default: `2`, i.e. the worker
    /// waits until the second assistant response has landed).
    #[serde(default = "default_titling_min_turns")]
    pub min_turns: i64,
    /// Length threshold above which a single first user message is
    /// considered "substantive enough to title immediately" (default: `200`).
    #[serde(default = "default_long_first_message_chars")]
    pub long_first_message_chars: usize,
}

fn default_titling_min_turns() -> i64 {
    2
}

fn default_long_first_message_chars() -> usize {
    200
}

impl Default for TitlingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_turns: default_titling_min_turns(),
            long_first_message_chars: default_long_first_message_chars(),
        }
    }
}

// ── Notifications ─────────────────────────────────────────────────────────────

/// Push notification / VAPID configuration (`[notifications]` section).
///
/// VAPID keys are generated on first `assistant webui serve` startup and
/// written back to `~/.assistant/config.toml`. Keeping them stable ensures
/// all existing browser push subscriptions remain valid.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationsConfig {
    /// Base64url-encoded VAPID private key (P-256). Auto-generated if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vapid_private_key: Option<String>,
    /// Base64url-encoded VAPID public key (P-256). Auto-generated if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vapid_public_key: Option<String>,
}
