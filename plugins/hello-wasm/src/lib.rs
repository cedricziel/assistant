use extism_pdk::*;

/// Entry point called by the assistant's WASM executor.
///
/// Input:  JSON-encoded skill params (e.g. `{"name": "World"}`)
/// Output: A greeting string
#[plugin_fn]
pub fn run(input: String) -> FnResult<String> {
    // Parse params — tolerate missing or non-object JSON gracefully
    let params: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("World");

    Ok(format!("Hello, {name}! (from WASM)"))
}
