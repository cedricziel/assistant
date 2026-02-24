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
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "HTTP or HTTPS URL to fetch"},
                "max_chars": {"type": "number", "description": "Max characters to return (default: 8000)"}
            },
            "required": ["url"]
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

    let body_selector = Selector::parse("body").unwrap();
    let skip_selector = Selector::parse("script, style, noscript, head").unwrap();

    let root = if let Some(body) = document.select(&body_selector).next() {
        body
    } else {
        return document
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    };

    let skip_ids: std::collections::HashSet<_> =
        root.select(&skip_selector).map(|el| el.id()).collect();

    let mut text_parts: Vec<&str> = Vec::new();
    for node in root.descendants() {
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
