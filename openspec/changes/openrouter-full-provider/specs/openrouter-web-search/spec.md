# Spec Delta

## Purpose

Lets OpenRouter models search the web through OpenRouter's `web` plugin, replacing the local web-search tool while it is on, and keeps the sources visible to the user.

## ADDED Requirements

### Requirement: Web plugin as a hosted tool

`[llm.openrouter.web_search]` SHALL accept the following:

- `enabled` (default `false`)
- `engine`: one of `native`, `exa`, `firecrawl`, `parallel`, `perplexity`; when unset, OpenRouter chooses.
- `max_results`
- `search_prompt`
- `include_domains`
- `exclude_domains`
- `search_context_size`: one of `low`, `medium`, `high`; native engine only.

When web search is enabled, requests SHALL include `plugins: [{"id": "web", ...}]` with the configured options. The provider SHALL report web search as a hosted tool, so the local web-search tool is not offered to the model. Setting `search_context_size` with a non-native engine SHALL be rejected at load time.

#### Scenario: Enabled with Exa

- **WHEN** `[llm.openrouter.web_search] enabled = true`, `engine = "exa"` and `max_results = 3` are set
- **THEN** requests include `"plugins": [{"id": "web", "engine": "exa", "max_results": 3}]`, and the local web-search tool is not offered to the model

#### Scenario: Disabled

- **WHEN** web search is not enabled
- **THEN** no `web` plugin is sent, and the local web-search tool stays available

### Requirement: Citations are kept

When a response carries `url_citation` annotations, the final answer SHALL end with a "Sources" list. The list SHALL contain each distinct cited URL once, in order of first citation, as a markdown link labelled with its title (or the URL when the title is missing). A URL already linked inline in the answer SHALL NOT be listed again.

#### Scenario: Two citations of one page

- **WHEN** the answer cites `https://a.example` twice and `https://b.example` once, and none of them is linked inline
- **THEN** the answer ends with a Sources list of two links, `a.example` first
