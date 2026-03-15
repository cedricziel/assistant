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

impl ToolSpec {
    /// Return `params_schema` normalised to a proper JSON Schema object.
    ///
    /// All LLM providers require a top-level `"type": "object"` schema for
    /// tool parameters. This method ensures the schema is well-formed
    /// regardless of how the tool handler originally declared it:
    ///
    /// - Already `{"type": "object", ...}` → returned as-is.
    /// - A bare JSON object (properties without a `type` key) → wrapped in
    ///   `{"type": "object", "properties": <schema>}`.
    /// - Anything else → an empty object schema with no properties.
    pub fn normalized_params_schema(&self) -> serde_json::Value {
        let schema = &self.params_schema;

        if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
            schema.clone()
        } else if schema.as_object().is_some() {
            serde_json::json!({"type": "object", "properties": schema})
        } else {
            serde_json::json!({"type": "object", "properties": {}, "required": []})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ToolSpec;
    use serde_json::json;

    fn make_tool(params_schema: serde_json::Value) -> ToolSpec {
        ToolSpec {
            name: "test-tool".into(),
            description: "A test tool".into(),
            params_schema,
            is_mutating: false,
            requires_confirmation: false,
        }
    }

    #[test]
    fn normalized_passes_through_object_schema() {
        let schema =
            json!({"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a"]});
        let tool = make_tool(schema.clone());
        assert_eq!(tool.normalized_params_schema(), schema);
    }

    #[test]
    fn normalized_wraps_bare_properties() {
        let schema = json!({"a": {"type": "string"}});
        let tool = make_tool(schema.clone());
        assert_eq!(
            tool.normalized_params_schema(),
            json!({"type": "object", "properties": schema})
        );
    }

    #[test]
    fn normalized_returns_empty_for_non_object() {
        let tool = make_tool(json!(null));
        assert_eq!(
            tool.normalized_params_schema(),
            json!({"type": "object", "properties": {}, "required": []})
        );
    }

    #[test]
    fn normalized_returns_empty_for_array() {
        let tool = make_tool(json!([1, 2, 3]));
        assert_eq!(
            tool.normalized_params_schema(),
            json!({"type": "object", "properties": {}, "required": []})
        );
    }
}
