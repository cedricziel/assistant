/// A provider-agnostic tool specification used at the LLM boundary.
///
/// Both Ollama and Anthropic providers convert this to their respective
/// wire formats internally.  Callers never need to know which provider
/// is in use.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    /// Tool name (kebab-case, e.g. "file-read").
    pub name: String,
    /// Short description of what this tool does (1-2 sentences).
    pub description: String,
    /// Full JSON Schema object for the tool's parameters.
    ///
    /// Must be a proper JSON Schema with `"type": "object"`, `"properties"`,
    /// and `"required"`.
    pub params_schema: serde_json::Value,
    /// Whether this tool mutates state (used for SafetyGate).
    pub is_mutating: bool,
    /// Whether the user must confirm before this tool runs.
    pub requires_confirmation: bool,
}
