//! Builtin handler for web-fetch skill — fetches a URL and returns stripped text.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
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
impl SkillHandler for WebFetchHandler {
    fn skill_name(&self) -> &str {
        "web-fetch"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let url = match params.get("url").and_then(|v| v.as_str()) {
            Some(u) => u.to_string(),
            None => {
                return Ok(SkillOutput::error("Missing required parameter 'url'"));
            }
        };

        // Validate URL scheme
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(SkillOutput::error(format!(
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
                return Ok(SkillOutput::error(format!(
                    "Failed to fetch '{}': {}",
                    url, e
                )));
            }
        };

        let status = response.status();
        if !status.is_success() {
            return Ok(SkillOutput::error(format!(
                "HTTP {} fetching '{}'",
                status, url
            )));
        }

        let html_body = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return Ok(SkillOutput::error(format!(
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

        Ok(SkillOutput::success(output))
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
