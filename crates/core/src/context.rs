use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use crate::AssistantConfig;

pub fn default_agent_id() -> &'static str {
    "default"
}

pub fn validate_agent_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn agent_base_dir(agent_id: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".assistant").join("agents").join(agent_id)
}

pub fn default_workspace_dir(agent_id: &str) -> PathBuf {
    agent_base_dir(agent_id).join("workspace")
}

pub fn set_runtime_agent_root(path: PathBuf) {
    let lock = RUNTIME_AGENT_ROOT.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = lock.write() {
        *guard = Some(path);
    }
}

pub fn runtime_agent_root() -> Option<PathBuf> {
    let lock = RUNTIME_AGENT_ROOT.get_or_init(|| RwLock::new(None));
    lock.read().ok().and_then(|guard| guard.clone())
}

pub fn set_runtime_workspace_dir(path: PathBuf) {
    let lock = RUNTIME_WORKSPACE_DIR.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = lock.write() {
        *guard = Some(path);
    }
}

pub fn runtime_workspace_dir() -> Option<PathBuf> {
    let lock = RUNTIME_WORKSPACE_DIR.get_or_init(|| RwLock::new(None));
    lock.read().ok().and_then(|guard| guard.clone())
}

pub fn apply_agent_context(config: &mut AssistantConfig, agent_id: &str) {
    let root = agent_base_dir(agent_id);
    config.agent.id = agent_id.to_string();

    config.memory.agents_path = Some(root.join("AGENTS.md").to_string_lossy().to_string());
    config.memory.soul_path = Some(root.join("SOUL.md").to_string_lossy().to_string());
    config.memory.identity_path = Some(root.join("IDENTITY.md").to_string_lossy().to_string());
    config.memory.user_path = Some(root.join("USER.md").to_string_lossy().to_string());
    config.memory.tools_path = Some(root.join("TOOLS.md").to_string_lossy().to_string());
    config.memory.memory_path = Some(root.join("MEMORY.md").to_string_lossy().to_string());
    config.memory.notes_dir = Some(root.join("memory").to_string_lossy().to_string());
    config.memory.bootstrap_path = Some(root.join("BOOTSTRAP.md").to_string_lossy().to_string());
    config.memory.heartbeat_path = Some(root.join("HEARTBEAT.md").to_string_lossy().to_string());
    config.memory.boot_path = Some(root.join("BOOT.md").to_string_lossy().to_string());
}

static RUNTIME_AGENT_ROOT: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
static RUNTIME_WORKSPACE_DIR: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
