//! The example config files shipped at the repo root must stay loadable.

use assistant_core::types::agent::{OrgConfig, ServerConfig};
use assistant_core::types::llm::LlmProviderKind;

#[test]
fn server_toml_example_parses() {
    toml::from_str::<ServerConfig>(include_str!("../../../server.toml")).unwrap();
}

#[test]
fn org_toml_example_parses_with_the_ollama_default() {
    let org: OrgConfig = toml::from_str(include_str!("../../../org.toml")).unwrap();
    assert_eq!(org.llm.provider, LlmProviderKind::Ollama);
}
