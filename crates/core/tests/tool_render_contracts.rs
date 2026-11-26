//! Contract harness for `schemas/tool-results/`.
//!
//! Every kind in [`ToolRenderData::KINDS`] needs a JSON Schema, at least one
//! valid and one invalid fixture, and a matching enum variant. The Dart
//! package checks the same fixtures (`contract_fixtures_test.dart`).

use std::fs;
use std::path::{Path, PathBuf};

use assistant_core::{ToolOutput, ToolRenderData};
use serde_json::Value;

fn schemas_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/tool-results")
}

fn fixtures(kind: &str, prefix: &str) -> Vec<PathBuf> {
    let dir = schemas_dir().join("fixtures").join(kind);
    let mut found: Vec<PathBuf> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().is_some_and(|ext| ext == "json")
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with(prefix))
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

#[test]
fn every_kind_has_a_schema_and_fixtures() {
    assert!(!ToolRenderData::KINDS.is_empty(), "KINDS must not be empty");
    for kind in ToolRenderData::KINDS {
        let schema = schemas_dir().join(format!("{kind}.schema.json"));
        assert!(schema.is_file(), "missing schema for {kind}: {schema:?}");
        assert!(
            !fixtures(kind, "valid-").is_empty(),
            "{kind} needs at least one valid-*.json fixture"
        );
        assert!(
            !fixtures(kind, "invalid-").is_empty(),
            "{kind} needs at least one invalid-*.json fixture"
        );
    }
}

#[test]
fn every_fixture_directory_is_a_registered_kind() {
    let mut dirs: Vec<String> = fs::read_dir(schemas_dir().join("fixtures"))
        .expect("fixtures dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    dirs.sort();
    let mut kinds: Vec<String> = ToolRenderData::KINDS
        .iter()
        .map(|k| k.to_string())
        .collect();
    kinds.sort();
    assert_eq!(
        dirs, kinds,
        "fixture directories must match ToolRenderData::KINDS"
    );
}

#[test]
fn valid_fixtures_pass_the_schema_and_deserialize_into_the_variant() {
    for kind in ToolRenderData::KINDS {
        let schema = ToolRenderData::schema(kind)
            .unwrap_or_else(|| panic!("ToolRenderData::schema({kind}) returned None"));
        let validator = jsonschema::validator_for(&schema)
            .unwrap_or_else(|e| panic!("{kind} schema does not compile: {e}"));
        for path in fixtures(kind, "valid-") {
            let doc = read_json(&path);
            let errors: Vec<String> = validator.iter_errors(&doc).map(|e| e.to_string()).collect();
            assert!(errors.is_empty(), "{path:?} fails its schema: {errors:?}");
            let parsed: ToolRenderData = serde_json::from_value(doc)
                .unwrap_or_else(|e| panic!("{path:?} does not deserialize: {e}"));
            assert_eq!(
                parsed.kind(),
                *kind,
                "{path:?} parsed into the wrong variant"
            );
        }
    }
}

#[test]
fn invalid_fixtures_fail_the_schema() {
    for kind in ToolRenderData::KINDS {
        let schema = ToolRenderData::schema(kind)
            .unwrap_or_else(|| panic!("ToolRenderData::schema({kind}) returned None"));
        let validator = jsonschema::validator_for(&schema)
            .unwrap_or_else(|e| panic!("{kind} schema does not compile: {e}"));
        for path in fixtures(kind, "invalid-") {
            let doc = read_json(&path);
            assert!(
                !validator.is_valid(&doc),
                "{path:?} should fail the {kind} schema but passes"
            );
        }
    }
}

#[test]
fn schema_lookup_rejects_unknown_kinds() {
    assert!(ToolRenderData::schema("weather").is_none());
}

#[test]
fn schema_files_on_disk_match_the_embedded_schemas() {
    for kind in ToolRenderData::KINDS {
        let on_disk = read_json(&schemas_dir().join(format!("{kind}.schema.json")));
        assert_eq!(ToolRenderData::schema(kind), Some(on_disk), "{kind}");
    }
}

#[test]
fn with_render_sets_tagged_data() {
    let data = ToolRenderData::Sources(assistant_core::SourcesData {
        query: "rust".into(),
        results: vec![],
    });
    let out = ToolOutput::success("ok").with_render(data);
    assert_eq!(
        out.data.as_ref().and_then(|d| d.get("kind")),
        Some(&Value::String("sources".into()))
    );
}
