//! Builtin handler for web-search tool — searches the web via DuckDuckGo.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use scraper::{Html, Selector};

const DEFAULT_NUM_RESULTS: usize = 10;

pub struct WebSearchHandler {
    client: reqwest::Client,
}

impl WebSearchHandler {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; AssistantBot/1.0)")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl Default for WebSearchHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for WebSearchHandler {
    fn name(&self) -> &str {
        "web-search"
    }

    fn description(&self) -> &str {
        "Search the web via DuckDuckGo and return a list of results with titles, URLs, and snippets."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query"},
                "num_results": {"type": "number", "description": "Max results to return (default: 10)"}
            },
            "required": ["query"]
        })
    }

    fn output_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "results": {
                    "type": "array",
                    "description": "Search result items",
                    "items": {
                        "type": "object",
                        "properties": {
                            "title":   {"type": "string"},
                            "url":     {"type": "string"},
                            "snippet": {"type": ["string", "null"]}
                        },
                        "required": ["title", "url"]
                    }
                }
            },
            "required": ["results"]
        }))
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let query = match params.get("query").and_then(|v| v.as_str()) {
            Some(q) => q.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'query'")),
        };

        let num_results = params
            .get("num_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_NUM_RESULTS);

        let response = match self
            .client
            .get("https://html.duckduckgo.com/html/")
            .query(&[("q", &query)])
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to reach DuckDuckGo: {}",
                    e
                )))
            }
        };

        if !response.status().is_success() {
            return Ok(ToolOutput::error(format!(
                "DuckDuckGo returned HTTP {}",
                response.status()
            )));
        }

        let body = match response.text().await {
            Ok(t) => t,
            Err(e) => return Ok(ToolOutput::error(format!("Failed to read response: {}", e))),
        };

        let results = parse_ddg_results(&body, num_results);

        if results.is_empty() {
            let data = serde_json::json!({"results": []});
            return Ok(
                ToolOutput::success(format!("No results found for query: {}", query))
                    .with_data(data),
            );
        }

        let formatted: Vec<String> = results
            .iter()
            .map(|(title, url, snippet)| {
                if let Some(s) = snippet {
                    format!("**{}**\n{}\n{}", title, url, s)
                } else {
                    format!("**{}**\n{}", title, url)
                }
            })
            .collect();

        let output = format!(
            "Search results for: {}\n\n{}",
            query,
            formatted.join("\n\n")
        );

        let data = serde_json::json!({
            "results": results.iter().map(|(title, url, snippet)| serde_json::json!({
                "title":   title,
                "url":     url,
                "snippet": snippet
            })).collect::<Vec<_>>()
        });

        Ok(ToolOutput::success(output).with_data(data))
    }
}

/// Parse DuckDuckGo HTML search results into structured `(title, url, snippet)` tuples.
fn parse_ddg_results(html: &str, limit: usize) -> Vec<(String, String, Option<String>)> {
    let document = Html::parse_document(html);

    let result_sel = Selector::parse(".result, .web-result").unwrap();
    let title_sel = Selector::parse("a.result__a").unwrap();
    let snippet_sel = Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();

    for result_el in document.select(&result_sel) {
        if results.len() >= limit {
            break;
        }

        let title_el = result_el.select(&title_sel).next();
        let snippet_el = result_el.select(&snippet_sel).next();

        let title = title_el
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty());

        let href = title_el
            .and_then(|el| el.value().attr("href"))
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty() && !s.starts_with("//duckduckgo.com"));

        let snippet = snippet_el
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty());

        if let (Some(t), Some(u)) = (title, href) {
            results.push((t, u, snippet));
        }
    }

    results
}
