//! Manager behaviour against a stdio MCP server implemented as a POSIX shell
//! loop, so the child-process transport and its environment handling run for
//! real.

#![cfg(unix)]

use std::collections::HashMap;

use assistant_core::ToolHandler;
use assistant_core::types::skills_mcp::{McpServerEntry, McpTransportConfig, McpTrustLevel};
use assistant_mcp_client::manager::McpClientManager;

/// Answers `initialize` and `tools/list`; the single tool's description is the
/// value of `$GREETING`, which lets a test observe the child's environment.
const SERVER_SCRIPT: &str = r#"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
  version=$(printf '%s' "$line" | sed -n 's/.*"protocolVersion":"\([^"]*\)".*/\1/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"%s","capabilities":{"tools":{}},"serverInfo":{"name":"sh","version":"1"}}}\n' "$id" "$version";;
    *'"method":"tools/list"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"greet","description":"%s","inputSchema":{"type":"object"}}]}}\n' "$id" "$GREETING";;
  esac
done
"#;

fn shell_entry(env: HashMap<String, String>) -> McpServerEntry {
    McpServerEntry {
        name: "shell".to_string(),
        transport: McpTransportConfig::Stdio {
            command: vec!["sh".into(), "-c".into(), SERVER_SCRIPT.into()],
        },
        env,
        enabled: true,
        trust: McpTrustLevel::Trust,
    }
}

#[tokio::test]
async fn stdio_server_tools_are_discovered_and_env_references_expanded() {
    // SAFETY: variable name is unique to this test.
    unsafe { std::env::set_var("MCP_CLIENT_TEST_NAME", "world") };
    let env = HashMap::from([(
        "GREETING".to_string(),
        "hello ${MCP_CLIENT_TEST_NAME}".to_string(),
    )]);

    let mgr = McpClientManager::start(&[shell_entry(env)]).await.unwrap();

    unsafe { std::env::remove_var("MCP_CLIENT_TEST_NAME") };
    let handlers = mgr.tool_handlers().await;
    let seen: Vec<(String, String)> = handlers
        .iter()
        .map(|h| (h.name().to_string(), h.description().to_string()))
        .collect();
    mgr.shutdown().await.unwrap();
    assert_eq!(
        (seen, mgr.server_count().await),
        (
            vec![("mcp__shell__greet".to_string(), "hello world".to_string())],
            1
        )
    );
}

#[tokio::test]
async fn unset_env_reference_expands_to_empty_string_for_the_child() {
    let env = HashMap::from([(
        "GREETING".to_string(),
        "hi${MCP_CLIENT_TEST_DEFINITELY_UNSET}there".to_string(),
    )]);

    let mgr = McpClientManager::start(&[shell_entry(env)]).await.unwrap();

    let description = mgr.tool_handlers().await[0].description().to_string();
    mgr.shutdown().await.unwrap();
    assert_eq!(description, "hithere");
}
