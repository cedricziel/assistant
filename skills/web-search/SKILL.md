---
name: web-search
description: >
  Search the web using DuckDuckGo and return a structured list of results
  (titles, URLs, and snippets). No API key required.
license: Apache-2.0
compatibility: Requires internet access
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"query": {"type": "string", "description": "The search query"}, "num_results": {"type": "number", "description": "Maximum number of results to return (default: 10)"}}'
---

## Instructions

Search the web via DuckDuckGo and return titles, URLs, and snippets for the
top results.

### Parameters

- `query` (string, required): The search query.
- `num_results` (number, optional): Maximum results to return. Defaults to 10.

### Behavior

- Queries DuckDuckGo and returns structured results
- Each result includes: title, URL, and a short snippet
- Returns an error if the network is unavailable or the query fails

### When to use

- Finding up-to-date information not in the model's training data
- Researching documentation, libraries, or recent events
- Verifying facts or finding official sources

### Example interactions

- "What's the latest Rust stable release?" → query: "Rust stable release 2025"
- "Search for Tokio async runtime docs" → query: "Tokio async runtime documentation"
