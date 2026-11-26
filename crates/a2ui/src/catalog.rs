//! The assistant component catalog and per-component schema validation.
//!
//! The catalog is the A2UI v1.0 basic catalog without Video, AudioPlayer and
//! Modal, with `common_types.json` inlined, shipped under its own id.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use anyhow::{Context, Result, anyhow};
use jsonschema::Validator;
use serde::Serialize;
use serde_json::{Map, Value};

/// Catalog id every `createSurface` names.
pub const CATALOG_ID: &str = "https://assistant.invalid/a2ui/catalogs/assistant/v1/catalog.json";

const CATALOG_JSON: &str = include_str!("../catalog/assistant-v1.json");

static EMBEDDED: OnceLock<std::result::Result<Catalog, String>> = OnceLock::new();

/// A validation failure in A2UI's error shape.
///
/// `path` is a JSON Pointer relative to the value that was validated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

impl ValidationIssue {
    pub fn new(code: &str, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            path: path.into(),
            message: message.into(),
        }
    }
}

/// The parsed catalog plus one compiled JSON Schema validator per component.
pub struct Catalog {
    doc: Value,
    validators: BTreeMap<String, Validator>,
}

impl Catalog {
    /// The catalog embedded in the binary, parsed and compiled once.
    pub fn embedded() -> Result<&'static Catalog> {
        EMBEDDED
            .get_or_init(|| Catalog::parse(CATALOG_JSON).map_err(|e| format!("{e:#}")))
            .as_ref()
            .map_err(|e| anyhow!("embedded A2UI catalog is invalid: {e}"))
    }

    fn parse(json: &str) -> Result<Catalog> {
        let doc: Value = serde_json::from_str(json).context("parse catalog JSON")?;
        let components = object_at(&doc, "components")?;
        let mut validators = BTreeMap::new();
        for name in components.keys() {
            // Validate against `#/components/<name>`; the rest of the catalog
            // stays in the root so its `#/$defs/...` references resolve.
            let mut schema = doc.clone();
            schema
                .as_object_mut()
                .context("catalog root is not an object")?
                .insert(
                    "$ref".to_owned(),
                    Value::String(format!("#/components/{name}")),
                );
            let validator = jsonschema::validator_for(&schema)
                .map_err(|e| anyhow!("compile schema for component {name}: {e}"))?;
            validators.insert(name.clone(), validator);
        }
        Ok(Catalog { doc, validators })
    }

    /// The catalog id declared by the document.
    pub fn catalog_id(&self) -> &str {
        self.doc
            .get("catalogId")
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// Component type names in the catalog.
    pub fn component_names(&self) -> Vec<String> {
        self.validators.keys().cloned().collect()
    }

    /// Function names in the catalog.
    pub fn function_names(&self) -> Vec<String> {
        object_at(&self.doc, "functions")
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Validate one component object against its catalog schema.
    ///
    /// Returns every issue found; an empty list means the component is valid.
    /// Paths are relative to the component (for example `/variant`).
    pub fn validate_component(&self, component: &Value) -> Vec<ValidationIssue> {
        let Some(obj) = component.as_object() else {
            return vec![ValidationIssue::new(
                "SCHEMA",
                "",
                "component must be a JSON object",
            )];
        };
        let Some(kind) = obj.get("component").and_then(Value::as_str) else {
            return vec![ValidationIssue::new(
                "SCHEMA",
                "/component",
                "\"component\" is required and must be a string",
            )];
        };
        let Some(validator) = self.validators.get(kind) else {
            return vec![ValidationIssue::new(
                "UNKNOWN_COMPONENT",
                "/component",
                format!("unknown component type \"{kind}\""),
            )];
        };
        validator
            .iter_errors(component)
            .map(|e| ValidationIssue::new("SCHEMA", e.instance_path().to_string(), e.to_string()))
            .collect()
    }
}

fn object_at<'a>(doc: &'a Value, key: &str) -> Result<&'a Map<String, Value>> {
    doc.get(key)
        .and_then(Value::as_object)
        .with_context(|| format!("catalog has no `{key}` object"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::{Value, json};

    use super::*;

    const COMPONENTS_TXT: &str = include_str!("../catalog/components.txt");

    fn catalog() -> &'static Catalog {
        Catalog::embedded().expect("embedded catalog loads")
    }

    #[test]
    fn component_names_match_the_shared_fixture() {
        let fixture: BTreeSet<String> = COMPONENTS_TXT
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_owned)
            .collect();
        let embedded: BTreeSet<String> = catalog().component_names().into_iter().collect();
        assert_eq!(embedded, fixture);
        assert_eq!(embedded.len(), 15);
    }

    #[test]
    fn excluded_basic_components_are_absent() {
        let names = catalog().component_names();
        for excluded in ["Video", "AudioPlayer", "Modal"] {
            assert!(!names.iter().any(|n| n == excluded), "{excluded} present");
        }
    }

    #[test]
    fn function_names_are_the_assistant_set() {
        let names: BTreeSet<String> = catalog().function_names().into_iter().collect();
        let expected: BTreeSet<String> = [
            "required",
            "regex",
            "length",
            "numeric",
            "email",
            "formatString",
            "formatNumber",
            "formatCurrency",
            "formatDate",
            "pluralize",
            "and",
            "or",
            "not",
            "openUrl",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(names, expected);
    }

    #[test]
    fn catalog_id_constant_matches_the_document() {
        assert_eq!(catalog().catalog_id(), CATALOG_ID);
    }

    #[test]
    fn valid_text_component_passes() {
        let issues = catalog().validate_component(&json!({
            "id": "t", "component": "Text", "text": "hello", "variant": "body"
        }));
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn every_component_type_has_a_working_validator() {
        let issues = catalog().validate_component(&json!({
            "id": "d", "component": "Divider"
        }));
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn unknown_component_type_is_reported() {
        let issues = catalog().validate_component(&json!({"id": "x", "component": "Script"}));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "UNKNOWN_COMPONENT");
        assert_eq!(issues[0].path, "/component");
        assert!(issues[0].message.contains("Script"));
    }

    #[test]
    fn excluded_component_type_is_reported_as_unknown() {
        let issues = catalog().validate_component(&json!({"id": "x", "component": "Video"}));
        assert_eq!(issues[0].code, "UNKNOWN_COMPONENT");
    }

    #[test]
    fn missing_required_property_is_reported() {
        let issues = catalog().validate_component(&json!({"id": "t", "component": "Text"}));
        assert!(!issues.is_empty());
        assert!(issues.iter().all(|i| i.code == "SCHEMA"));
        assert!(
            issues.iter().any(|i| i.message.contains("text")),
            "{issues:?}"
        );
    }

    #[test]
    fn wrong_enum_value_is_reported_with_its_path() {
        let issues = catalog().validate_component(&json!({
            "id": "t", "component": "Text", "text": "x", "variant": "headline"
        }));
        assert!(issues.iter().any(|i| i.path == "/variant"), "{issues:?}");
    }

    #[test]
    fn non_object_component_is_reported() {
        let issues = catalog().validate_component(&Value::String("Text".into()));
        assert_eq!(issues[0].code, "SCHEMA");
    }

    #[test]
    fn components_with_nested_references_validate() {
        let valid = [
            json!({"id": "b", "component": "Button", "child": "l", "variant": "primary",
                "action": {"event": {"name": "go", "context": {"v": {"@path": "/x"}}}}}),
            json!({"id": "f", "component": "TextField", "label": "Phone",
                "value": {"@path": "/phone"},
                "@checks": [{"condition": {"@call": "regex",
                    "args": {"value": {"@path": "/phone"}, "pattern": "^[0-9]{5}$"}},
                    "message": "Five digits"}]}),
            json!({"id": "i", "component": "Image", "url": "https://example.com/a.png"}),
            json!({"id": "t", "component": "Tabs", "tabs": [{"title": "One", "child": "c"}]}),
            json!({"id": "p", "component": "ChoicePicker", "variant": "mutuallyExclusive",
                "options": [{"label": "A", "value": "a"}], "value": {"@path": "/p"}}),
        ];
        for c in &valid {
            let issues = catalog().validate_component(c);
            assert!(issues.is_empty(), "{c}: {issues:?}");
        }
    }

    #[test]
    fn button_without_an_action_is_rejected() {
        let issues = catalog().validate_component(&json!({
            "id": "b", "component": "Button", "child": "l"
        }));
        assert!(
            issues.iter().any(|i| i.message.contains("action")),
            "{issues:?}"
        );
    }

    #[test]
    fn component_without_a_type_is_reported() {
        let issues = catalog().validate_component(&json!({"id": "t"}));
        assert_eq!(issues[0].path, "/component");
    }

    #[test]
    fn a_catalog_without_components_is_rejected() {
        assert!(Catalog::parse("{}").is_err());
        assert!(Catalog::parse("not json").is_err());
    }
}
