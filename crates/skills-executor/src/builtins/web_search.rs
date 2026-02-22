//! Builtin handler for web-search skill — searches the web via DuckDuckGo.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
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
            .expect("Failed to build HTTP client");
        Self { client }
    }
}

impl Default for WebSearchHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillHandler for WebSearchHandler {
    fn skill_name(&self) -> &str {
        "web-search"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let query = match params.get("query").and_then(|v| v.as_str()) {
            Some(q) => q.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'query'")),
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
                return Ok(SkillOutput::error(format!(
                    "Failed to reach DuckDuckGo: {}",
                    e
                )))
            }
        };

        if !response.status().is_success() {
            return Ok(SkillOutput::error(format!(
                "DuckDuckGo returned HTTP {}",
                response.status()
            )));
        }

        let body = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return Ok(SkillOutput::error(format!(
                    "Failed to read response: {}",
                    e
                )))
            }
        };

        let results = parse_ddg_results(&body, num_results);

        if results.is_empty() {
            return Ok(SkillOutput::success(format!(
                "No results found for query: {}",
                query
            )));
        }

        let output = format!("Search results for: {}\n\n{}", query, results.join("\n\n"));

        Ok(SkillOutput::success(output))
    }
}

/// Parse DuckDuckGo HTML search results.
fn parse_ddg_results(html: &str, limit: usize) -> Vec<String> {
    let document = Html::parse_document(html);

    // DuckDuckGo HTML endpoint result structure:
    // div.result / div.web-result → a.result__a (title + href), a.result__snippet (text)
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

        // Only include results that have at least a title and URL.
        if let (Some(t), Some(u)) = (title, href) {
            let entry = if let Some(s) = snippet {
                format!("**{}**\n{}\n{}", t, u, s)
            } else {
                format!("**{}**\n{}", t, u)
            };
            results.push(entry);
        }
    }

    results
}
