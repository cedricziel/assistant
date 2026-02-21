//! WASM-tier skill executor — loads an extism plugin and calls its `run` export.
//!
//! The plugin receives the skill parameters as a JSON string via the extism
//! input buffer and must return a UTF-8 string via the output buffer.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillOutput};
use tracing::debug;

/// Name of the exported function every WASM skill plugin must provide.
const WASM_ENTRY: &str = "run";

pub async fn run_wasm(
    plugin_path: &Path,
    params: &HashMap<String, serde_json::Value>,
    _ctx: &ExecutionContext,
) -> Result<SkillOutput> {
    if !plugin_path.exists() {
        return Ok(SkillOutput::error(format!(
            "WASM plugin not found: {}",
            plugin_path.display()
        )));
    }

    let params_json = serde_json::to_string(params)?;
    let plugin_path = plugin_path.to_path_buf();

    debug!(
        "wasm_executor: loading {:?}, params={}",
        plugin_path, params_json
    );

    // extism Plugin is not Send, so run in a blocking thread
    let output = tokio::task::spawn_blocking(move || {
        use extism::{Manifest, Plugin, Wasm};

        let wasm = Wasm::file(&plugin_path);
        let manifest = Manifest::new([wasm]);
        let mut plugin = Plugin::new(&manifest, [], true).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load WASM plugin '{}': {e}",
                plugin_path.display()
            )
        })?;

        let result: String = plugin
            .call::<&str, &str>(WASM_ENTRY, &params_json)
            .map_err(|e| anyhow::anyhow!("WASM plugin call '{}' failed: {e}", WASM_ENTRY))?
            .to_string();

        anyhow::Ok(result)
    })
    .await??;

    Ok(SkillOutput::success(output))
}
