//! Render data contracts for rich tool results.
//!
//! A tool may attach structured data to its [`ToolOutput`](crate::ToolOutput)
//! so a client can draw a dedicated view (terminal, sources, ...) instead of a
//! text blob. The data is a JSON object tagged by `kind`; the JSON Schema files
//! under `schemas/tool-results/` are the source of truth and are shared with the
//! Dart client through the fixtures next to them. See
//! `schemas/tool-results/README.md`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// -- Contract ----------------------------------------------------------------

/// Structured render data, tagged by `kind` on the wire.
///
/// Contracts evolve in place (no version field); a client that cannot parse a
/// payload falls back to its generic view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolRenderData {
    /// Result of a shell command.
    Terminal(TerminalData),
    /// Search results with titles, URLs and snippets.
    Sources(SourcesData),
}

impl ToolRenderData {
    /// Every `kind` that has a contract. Each one needs a schema file and
    /// fixtures (`crates/core/tests/tool_render_contracts.rs`).
    pub const KINDS: &'static [&'static str] = &["terminal", "sources"];

    /// The wire `kind` of this payload.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Terminal(_) => "terminal",
            Self::Sources(_) => "sources",
        }
    }

    /// The JSON Schema (2020-12) for `kind`, or `None` for an unknown kind.
    pub fn schema(kind: &str) -> Option<Value> {
        let text = match kind {
            "terminal" => {
                include_str!("../../../schemas/tool-results/terminal.schema.json")
            }
            "sources" => {
                include_str!("../../../schemas/tool-results/sources.schema.json")
            }
            _ => return None,
        };
        serde_json::from_str(text).ok()
    }
}

/// `terminal` contract: a finished shell command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminalData {
    pub command: String,
    /// `None` (JSON `null`) when the command timed out.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub duration_ms: u64,
    pub timed_out: bool,
}

/// `sources` contract: results of a web search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourcesData {
    pub query: String,
    pub results: Vec<SourceResult>,
}

/// One entry of [`SourcesData::results`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceResult {
    pub title: String,
    pub url: String,
    pub domain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_out_terminal_serializes_a_null_exit_code() {
        let data = ToolRenderData::Terminal(TerminalData {
            command: "sleep 9".into(),
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 1000,
            timed_out: true,
        });
        let value = serde_json::to_value(&data).unwrap();
        assert_eq!(value["kind"], "terminal");
        assert!(
            value["exit_code"].is_null(),
            "exit_code key must be present"
        );
    }

    #[test]
    fn unknown_fields_are_ignored_on_deserialize() {
        let value = serde_json::json!({
            "kind": "sources", "query": "q", "results": [], "future_field": 1
        });
        let parsed: ToolRenderData = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.kind(), "sources");
    }

    #[test]
    fn kinds_match_variant_kinds() {
        assert_eq!(ToolRenderData::KINDS, &["terminal", "sources"]);
    }
}
