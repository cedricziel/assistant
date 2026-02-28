//! Configuration loading for the assistant.
//!
//! Provides shared helpers so every interface binary (CLI, web-ui, Slack, …)
//! loads `~/.assistant/config.toml` the same way.

use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::AssistantConfig;

/// Return the default path to the assistant config file
/// (`~/.assistant/config.toml`).
///
/// Returns `None` if the home directory cannot be determined.
pub fn default_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".assistant").join("config.toml"))
}

/// Load [`AssistantConfig`] from a TOML file at `path`.
///
/// * If the file does not exist, returns [`AssistantConfig::default()`].
/// * If the file cannot be read or parsed, logs a warning and returns
///   the default.
///
/// This is intentionally synchronous — config loading happens once at
/// startup and the file is tiny, so blocking I/O is acceptable and avoids
/// requiring a tokio runtime.
pub fn load_config(path: &Path) -> AssistantConfig {
    if !path.exists() {
        info!("No config file at {}; using defaults", path.display());
        return AssistantConfig::default();
    }

    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to read {}: {}", path.display(), e);
            return AssistantConfig::default();
        }
    };

    match toml::from_str::<AssistantConfig>(&raw) {
        Ok(cfg) => {
            info!(
                "Loaded config from {} (provider={:?}, model={})",
                path.display(),
                cfg.llm.provider,
                cfg.llm.model,
            );
            cfg
        }
        Err(e) => {
            warn!("Failed to parse {}: {}; using defaults", path.display(), e);
            AssistantConfig::default()
        }
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config_path_ends_with_config_toml() {
        if let Some(p) = default_config_path() {
            assert!(p.ends_with("config.toml"));
            assert!(p.to_string_lossy().contains(".assistant"));
        }
        // If home_dir() returns None (CI), the test is a no-op.
    }

    #[test]
    fn test_load_config_missing_file_returns_default() {
        let cfg = load_config(Path::new("/nonexistent/path/config.toml"));
        // Should be the default — Ollama provider
        assert_eq!(cfg.llm.provider, crate::LlmProviderKind::Ollama);
    }

    #[test]
    fn test_load_config_parses_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
"#
        )
        .unwrap();

        let cfg = load_config(&path);
        assert_eq!(cfg.llm.provider, crate::LlmProviderKind::Anthropic);
        assert_eq!(cfg.llm.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_load_config_invalid_toml_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "{{{{not valid toml").unwrap();

        let cfg = load_config(&path);
        assert_eq!(cfg.llm.provider, crate::LlmProviderKind::Ollama);
    }
}
