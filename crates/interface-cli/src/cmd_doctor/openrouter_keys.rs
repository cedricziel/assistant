//! Which OpenRouter-backed features are configured, where each one's key
//! comes from, and whether OpenRouter accepts it.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use assistant_core::Secret;
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::llm::{
    EmbeddingProviderKind, KeySource, LlmProviderKind, OPENROUTER_API_KEY_ENV,
    openrouter_key_with_source,
};
use assistant_core::types::transcription::{TranscriptionProviderKind, TtsProviderKind};
use assistant_llm_provider::openrouter::{DEFAULT_BASE_URL, KeyStatus, key_status};

use super::CheckResult;

const NAME: &str = "OpenRouter keys";

/// The base URL chat uses when `[llm] base_url` was left at Ollama's default.
const OLLAMA_DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// One OpenRouter-backed feature and the key it would use.
struct Feature {
    name: &'static str,
    section: &'static str,
    key: Option<(Secret, KeySource)>,
    base_url: String,
}

/// The configured OpenRouter-backed features, with their keys resolved the
/// way the runtime resolves them.
fn features(config: &OrgConfig, env: impl Fn(&str) -> Option<String> + Copy) -> Vec<Feature> {
    let llm = &config.llm;
    let resolve = |section: Option<&Secret>, env_var: &str| {
        openrouter_key_with_source(section, llm, env_var, env)
    };
    let base = |url: Option<&String>| url.cloned().unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
    let mut features = Vec::new();
    let chat_base = if llm.base_url == OLLAMA_DEFAULT_BASE_URL {
        DEFAULT_BASE_URL.to_owned()
    } else {
        llm.base_url.clone()
    };
    if llm.provider == LlmProviderKind::OpenRouter {
        features.push(Feature {
            name: "chat",
            section: "[llm]",
            key: resolve(None, OPENROUTER_API_KEY_ENV),
            base_url: chat_base.clone(),
        });
    }
    match &llm.embeddings {
        Some(emb) if emb.provider == EmbeddingProviderKind::OpenRouter => features.push(Feature {
            name: "embeddings",
            section: "[llm.embeddings]",
            key: resolve(emb.api_key.as_ref(), OPENROUTER_API_KEY_ENV),
            base_url: base(emb.base_url.as_ref()),
        }),
        None if llm.provider == LlmProviderKind::OpenRouter => features.push(Feature {
            name: "embeddings",
            section: "[llm]",
            key: resolve(None, OPENROUTER_API_KEY_ENV),
            base_url: chat_base,
        }),
        _ => {}
    }
    if let Some(t) = config
        .transcription
        .as_ref()
        .filter(|t| t.provider == TranscriptionProviderKind::OpenRouter)
    {
        features.push(Feature {
            name: "transcription",
            section: "[transcription]",
            key: resolve(t.api_key.as_ref(), OPENROUTER_API_KEY_ENV),
            base_url: base(t.base_url.as_ref()),
        });
    }
    if let Some(t) = config
        .tts
        .as_ref()
        .filter(|t| t.provider == TtsProviderKind::OpenRouter)
    {
        features.push(Feature {
            name: "tts",
            section: "[tts]",
            key: resolve(t.api_key.as_ref(), OPENROUTER_API_KEY_ENV),
            base_url: base(t.base_url.as_ref()),
        });
    }
    if let Some(images) = config.images.as_ref().filter(|i| i.enabled) {
        features.push(Feature {
            name: "images",
            section: "[images]",
            key: resolve(images.api_key.as_ref(), OPENROUTER_API_KEY_ENV),
            base_url: base(images.base_url.as_ref()),
        });
    }
    let decision = &config.permissions.decision_model;
    if decision.enabled {
        features.push(Feature {
            name: "decision model",
            section: "[permissions.decision_model]",
            key: resolve(decision.api_key.as_ref(), &decision.api_key_env),
            base_url: decision.base_url.clone(),
        });
    }
    features
}

/// Lists each OpenRouter-backed feature with its key's source and health.
/// Each distinct key is checked once; no key material is printed.
pub(super) async fn check_openrouter_keys(
    config: &OrgConfig,
    env: impl Fn(&str) -> Option<String> + Copy,
) -> CheckResult {
    let features = features(config, env);
    if features.is_empty() {
        return CheckResult::skipped(NAME, "No OpenRouter-backed features configured");
    }
    let mut statuses: HashMap<(String, String), KeyStatus> = HashMap::new();
    for feature in &features {
        if let Some((key, _)) = &feature.key {
            let id = (key.expose().to_owned(), feature.base_url.clone());
            if let Entry::Vacant(slot) = statuses.entry(id) {
                slot.insert(key_status(&feature.base_url, key).await);
            }
        }
    }

    let (mut errors, mut warnings) = (0, 0);
    let mut details = Vec::with_capacity(features.len());
    for feature in &features {
        let Some((key, source)) = &feature.key else {
            errors += 1;
            details.push(format!(
                "{}: no key; set api_key in {}, an OpenRouter [llm] api_key or {}",
                feature.name,
                feature.section,
                env_var_for(feature, config)
            ));
            continue;
        };
        let source = match source {
            KeySource::Section => feature.section.to_owned(),
            KeySource::Llm => "[llm]".to_owned(),
            KeySource::Env(var) => var.clone(),
        };
        let health = match statuses.get(&(key.expose().to_owned(), feature.base_url.clone())) {
            Some(KeyStatus::Valid {
                limit_remaining,
                free_tier,
            }) => {
                let limit = match limit_remaining {
                    Some(left) => format!("${left:.2} left"),
                    None => "no limit".to_owned(),
                };
                let tier = if *free_tier { ", free tier" } else { "" };
                format!("valid, {limit}{tier}")
            }
            Some(KeyStatus::Invalid) => {
                errors += 1;
                "OpenRouter key invalid or revoked".to_owned()
            }
            Some(KeyStatus::Unreachable(reason)) => {
                warnings += 1;
                format!("unreachable: {reason}")
            }
            None => {
                warnings += 1;
                "unreachable: not checked".to_owned()
            }
        };
        details.push(format!("{}: key from {source}, {health}", feature.name));
    }

    let summary = format!("{} feature(s) use OpenRouter", features.len());
    let mut result = if errors > 0 {
        CheckResult::error(NAME, summary)
    } else if warnings > 0 {
        CheckResult::warning(NAME, summary)
    } else {
        CheckResult::ok(NAME, summary)
    };
    for detail in details {
        result = result.with_detail(detail);
    }
    result
}

/// The environment variable `feature` falls back to.
fn env_var_for(feature: &Feature, config: &OrgConfig) -> String {
    if feature.name == "decision model" {
        config.permissions.decision_model.api_key_env.clone()
    } else {
        OPENROUTER_API_KEY_ENV.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_core::types::agent::OrgConfig;

    use super::*;
    use crate::cmd_doctor::CheckStatus;

    fn config(toml: &str, server: &MockServer) -> OrgConfig {
        toml::from_str(&toml.replace("BASE", &server.uri())).unwrap()
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    async fn mount_key(server: &MockServer, response: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path("/key"))
            .respond_with(response)
            .expect(1)
            .mount(server)
            .await;
    }

    const SHARED: &str = r#"
        [llm]
        provider = "openrouter"
        model = "openai/gpt-4o"
        api_key = "sk-or-secret"
        base_url = "BASE"

        [transcription]
        provider = "openrouter"
        base_url = "BASE"

        [tts]
        provider = "openrouter"
        base_url = "BASE"
    "#;

    #[tokio::test]
    async fn a_shared_key_is_checked_once_and_never_printed() {
        let server = MockServer::start().await;
        mount_key(
            &server,
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"limit_remaining": 12.5, "is_free_tier": false}})),
        )
        .await;

        let result = check_openrouter_keys(&config(SHARED, &server), no_env).await;

        assert_eq!(result.status, CheckStatus::Ok, "{result:?}");
        let details = result.details.join("\n");
        for feature in ["chat", "embeddings", "transcription", "tts"] {
            assert!(
                details.contains(&format!("{feature}: key from [llm]")),
                "{details}"
            );
        }
        assert!(details.contains("12.50"), "{details}");
        assert!(
            !format!("{result:?}").contains("sk-or-secret"),
            "no key material"
        );
    }

    #[tokio::test]
    async fn a_revoked_key_fails_every_feature_using_it() {
        let server = MockServer::start().await;
        mount_key(&server, ResponseTemplate::new(401)).await;

        let result = check_openrouter_keys(&config(SHARED, &server), no_env).await;

        assert_eq!(result.status, CheckStatus::Error);
        let failing = result
            .details
            .iter()
            .filter(|d| d.contains("OpenRouter key invalid or revoked"))
            .count();
        assert_eq!(
            failing, 4,
            "chat, embeddings, transcription, tts: {:?}",
            result.details
        );
    }

    #[tokio::test]
    async fn an_unreachable_key_check_is_a_warning() {
        let server = MockServer::start().await;
        mount_key(&server, ResponseTemplate::new(503)).await;

        let result = check_openrouter_keys(&config(SHARED, &server), no_env).await;

        assert_eq!(result.status, CheckStatus::Warning, "{result:?}");
        assert!(
            result.details.iter().all(|d| d.contains("unreachable")),
            "{:?}",
            result.details
        );
    }

    #[tokio::test]
    async fn sources_are_the_section_the_llm_key_or_the_env() {
        let server = MockServer::start().await;
        mount_key(
            &server,
            ResponseTemplate::new(200).set_body_json(json!({"data": {"is_free_tier": true}})),
        )
        .await;
        let toml = r#"
            [llm]
            provider = "anthropic"
            model = "claude"

            [tts]
            provider = "openrouter"
            api_key = "own"
            base_url = "BASE"

            [transcription]
            provider = "openrouter"
            base_url = "BASE"
        "#;
        let env = |name: &str| (name == "OPENROUTER_API_KEY").then(|| "own".to_string());

        let result = check_openrouter_keys(&config(toml, &server), env).await;

        let details = result.details.join("\n");
        assert!(details.contains("tts: key from [tts]"), "{details}");
        assert!(
            details.contains("transcription: key from OPENROUTER_API_KEY"),
            "{details}"
        );
        assert!(
            details.contains("no limit") && details.contains("free tier"),
            "{details}"
        );
        assert!(
            !details.contains("chat"),
            "Anthropic chat is not OpenRouter: {details}"
        );
    }

    #[tokio::test]
    async fn a_feature_without_a_key_is_an_error() {
        let server = MockServer::start().await;
        let toml = r#"
            [llm]
            provider = "ollama"
            model = "llama3"

            [tts]
            provider = "openrouter"
        "#;

        let result = check_openrouter_keys(&config(toml, &server), no_env).await;

        assert_eq!(result.status, CheckStatus::Error);
        assert!(
            result.details.join("\n").contains("tts: no key"),
            "{:?}",
            result.details
        );
    }

    #[tokio::test]
    async fn no_openrouter_features_skips_the_check() {
        let result = check_openrouter_keys(&OrgConfig::default(), no_env).await;
        assert_eq!(result.status, CheckStatus::Skipped);
    }
}
