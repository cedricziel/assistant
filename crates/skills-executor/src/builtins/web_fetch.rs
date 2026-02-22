//! Builtin handler for web-fetch tool — fetches a URL and returns stripped text.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use scraper::Html;

const DEFAULT_MAX_CHARS: usize = 8000;

pub struct WebFetchHandler {
    client: reqwest::Client,
}

impl WebFetchHandler {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; AssistantBot/1.0)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }
}

impl Default for WebFetchHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for WebFetchHandler {
    fn name(&self) -> &str {
        "web-fetch"
    }

    fn description(&self) -> &str {
        "Fetch the content of a URL and return it as plain text (HTML stripped)."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "url": {"type": "string", "description": "HTTP or HTTPS URL to fetch"},
            "max_chars": {"type": "number", "description": "Max characters to return (default: 8000)"}
        })
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let url = match params.get("url").and_then(|v| v.as_str()) {
            Some(u) => u.to_string(),
            None => {
                return Ok(ToolOutput::error("Missing required parameter 'url'"));
            }
        };

        // Validate URL scheme
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ToolOutput::error(format!(
                "Invalid URL '{}': must start with http:// or https://",
                url
            )));
        }

        let max_chars = params
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_MAX_CHARS);

        // Fetch the page
        let response = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to fetch '{}': {}",
                    url, e
                )));
            }
        };

        let status = response.status();
        if !status.is_success() {
            return Ok(ToolOutput::error(format!(
                "HTTP {} fetching '{}'",
                status, url
            )));
        }

        let html_body = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to read response body from '{}': {}",
                    url, e
                )));
            }
        };

        // Parse and strip HTML
        let document = Html::parse_document(&html_body);

        // Extract title
        let title = extract_title(&document);

        // Extract visible text content
        let text = extract_text(&document);

        // Truncate to max_chars
        let truncated = if text.len() > max_chars {
            let mut end = max_chars;
            // Walk back to a whitespace boundary to avoid splitting mid-word
            while end > 0 && !text.is_char_boundary(end) {
                end -= 1;
            }
            format!(
                "{}\n\n[Content truncated at {} characters]",
                &text[..end],
                max_chars
            )
        } else {
            text
        };

        let output = if let Some(t) = title {
            format!("Title: {}\nURL: {}\n\n{}", t, url, truncated)
        } else {
            format!("URL: {}\n\n{}", url, truncated)
        };

        Ok(ToolOutput::success(output))
    }
}

fn extract_title(document: &Html) -> Option<String> {
    use scraper::Selector;
    let selector = Selector::parse("title").ok()?;
    document
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_text(document: &Html) -> String {
    use scraper::Selector;

    // Remove script and style elements from consideration by walking the tree manually
    // scraper doesn't have a "exclude selector" so we walk element references
    let body_selector = Selector::parse("body").unwrap();
    let skip_selector = Selector::parse("script, style, noscript, head").unwrap();

    let root = if let Some(body) = document.select(&body_selector).next() {
        body
    } else {
        // Fall back to document root
        return document
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    };

    // Collect text from all descendants, skipping script/style subtrees
    let skip_ids: std::collections::HashSet<_> =
        root.select(&skip_selector).map(|el| el.id()).collect();

    let mut text_parts: Vec<&str> = Vec::new();
    for node in root.descendants() {
        // Check if any ancestor of this node is in the skip set
        if let Some(el) = scraper::ElementRef::wrap(node) {
            if skip_ids.contains(&el.id()) {
                continue;
            }
        }
        if let Some(text) = node.value().as_text() {
            let t = text.trim();
            if !t.is_empty() {
                text_parts.push(t);
            }
        }
    }

    text_parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use uuid::Uuid;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 1,
            interface: Interface::Cli,
            interactive: false,
        }
    }

    fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[tokio::test]
    async fn fetches_url_and_strips_html() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("<html><body>Hello world</body></html>"),
            )
            .mount(&server)
            .await;

        let handler = WebFetchHandler::new();
        let ctx = make_ctx();
        let p = params(&[("url", serde_json::Value::String(server.uri()))]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success, "Expected success, got: {}", result.content);
        assert!(
            result.content.contains("Hello world"),
            "Expected 'Hello world', got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn returns_error_for_404() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let handler = WebFetchHandler::new();
        let ctx = make_ctx();
        let p = params(&[("url", serde_json::Value::String(server.uri()))]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success, "Expected error for 404");
        assert!(
            result.content.contains("404"),
            "Expected '404' in error, got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn truncates_at_max_chars() {
        let server = MockServer::start().await;
        let long_text = "a".repeat(500);
        let body = format!("<html><body>{}</body></html>", long_text);
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;

        let handler = WebFetchHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            ("url", serde_json::Value::String(server.uri())),
            ("max_chars", serde_json::json!(100)),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success);
        assert!(
            result.content.contains("[Content truncated"),
            "Expected truncation marker, got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn invalid_url_scheme() {
        let handler = WebFetchHandler::new();
        let ctx = make_ctx();
        let p = params(&[(
            "url",
            serde_json::Value::String("ftp://example.com".to_string()),
        )]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("must start with http"),
            "Got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn missing_url_param() {
        let handler = WebFetchHandler::new();
        let ctx = make_ctx();
        let p = params(&[]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(result.content.contains("url"), "Got: {}", result.content);
    }

    #[tokio::test]
    async fn extracts_title_from_html() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "<html><head><title>My Page Title</title></head><body>Content here</body></html>",
            ))
            .mount(&server)
            .await;

        let handler = WebFetchHandler::new();
        let ctx = make_ctx();
        let p = params(&[("url", serde_json::Value::String(server.uri()))]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success);
        assert!(
            result.content.contains("Title: My Page Title"),
            "Expected title, got: {}",
            result.content
        );
    }

    #[test]
    fn self_describing() {
        let handler = WebFetchHandler::new();
        assert!(!handler.description().is_empty());
        assert!(handler.params_schema().is_object());
        assert!(
            !handler.is_mutating(),
            "WebFetchHandler should not be mutating"
        );
    }
}
