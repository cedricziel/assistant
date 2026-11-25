//! Config structs must never print their credentials through `{:?}`.

use assistant_core::types::channels::{
    MatrixConfig, MattermostConfig, NextcloudConfig, SignalConfig, SlackConfig,
};
use assistant_core::types::llm::{EmbeddingConfig, LlmConfig};
use assistant_core::types::storage::BusConfig;
use assistant_core::types::transcription::{TranscriptionConfig, TtsConfig};

const NEEDLE: &str = "needle-credential-9f3a";

fn assert_redacted(debug: String, what: &str) {
    assert!(
        !debug.contains(NEEDLE),
        "{what} Debug output leaked a credential: {debug}"
    );
    assert!(
        debug.contains("[REDACTED]"),
        "{what} Debug output should show the redaction marker: {debug}"
    );
}

#[test]
fn signal_config_debug_redacts() {
    let cfg: SignalConfig = toml::from_str(&format!("api_password = \"{NEEDLE}\"")).unwrap();
    assert_redacted(format!("{cfg:?}"), "SignalConfig");
}

#[test]
fn slack_config_debug_redacts() {
    let cfg: SlackConfig = toml::from_str(&format!(
        "bot_token = \"{NEEDLE}\"\napp_token = \"{NEEDLE}\""
    ))
    .unwrap();
    assert_redacted(format!("{cfg:?}"), "SlackConfig");
}

#[test]
fn mattermost_config_debug_redacts() {
    let cfg: MattermostConfig = toml::from_str(&format!("token = \"{NEEDLE}\"")).unwrap();
    assert_redacted(format!("{cfg:?}"), "MattermostConfig");
}

#[test]
fn matrix_config_debug_redacts() {
    let cfg: MatrixConfig = toml::from_str(&format!(
        "password = \"{NEEDLE}\"\naccess_token = \"{NEEDLE}\""
    ))
    .unwrap();
    assert_redacted(format!("{cfg:?}"), "MatrixConfig");
}

#[test]
fn nextcloud_config_debug_redacts() {
    let cfg: NextcloudConfig = toml::from_str(&format!("secret = \"{NEEDLE}\"")).unwrap();
    assert_redacted(format!("{cfg:?}"), "NextcloudConfig");
}

#[test]
fn llm_config_debug_redacts() {
    let cfg: LlmConfig = toml::from_str(&format!("api_key = \"{NEEDLE}\"")).unwrap();
    assert_redacted(format!("{cfg:?}"), "LlmConfig");
}

#[test]
fn embedding_config_debug_redacts() {
    let cfg: EmbeddingConfig =
        toml::from_str(&format!("provider = \"voyage\"\napi_key = \"{NEEDLE}\"")).unwrap();
    assert_redacted(format!("{cfg:?}"), "EmbeddingConfig");
}

#[test]
fn transcription_config_debug_redacts() {
    let cfg: TranscriptionConfig =
        toml::from_str(&format!("provider = \"whisper\"\napi_key = \"{NEEDLE}\"")).unwrap();
    assert_redacted(format!("{cfg:?}"), "TranscriptionConfig");
}

#[test]
fn tts_config_debug_redacts() {
    let cfg: TtsConfig =
        toml::from_str(&format!("provider = \"openai\"\napi_key = \"{NEEDLE}\"")).unwrap();
    assert_redacted(format!("{cfg:?}"), "TtsConfig");
}

#[test]
fn bus_config_debug_redacts() {
    let cfg: BusConfig =
        toml::from_str(&format!("password = \"{NEEDLE}\"\ntoken = \"{NEEDLE}\"")).unwrap();
    assert_redacted(format!("{cfg:?}"), "BusConfig");
}

#[test]
fn secrets_still_round_trip_through_toml() {
    let cfg: SlackConfig = toml::from_str(&format!("bot_token = \"{NEEDLE}\"")).unwrap();
    let out = toml::to_string(&cfg).unwrap();
    assert!(
        out.contains(NEEDLE),
        "serialization must keep the plain value for config files: {out}"
    );
}
