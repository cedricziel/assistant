---
name: web-fetch
description: >
  Fetch the content of a URL and return its text. Use when the user asks to read a webpage,
  retrieve online documentation, look up a specific URL, or get information from the web.
  Returns the page title and readable text content (HTML stripped).
license: Apache-2.0
compatibility: Requires network access
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"url": {"type": "string", "description": "The full URL to fetch (must start with http:// or https://)"}, "max_chars": {"type": "integer", "description": "Maximum characters to return (default: 8000)", "default": 8000}}'
---

## Instructions

Fetch the text content of a web page and return it for analysis or display.

### Parameters
- `url` (string, required): The full URL to fetch (must start with `http://` or `https://`)
- `max_chars` (integer, optional, default 8000): Truncate the returned content to this many characters

### Behavior
- Performs an HTTP GET request with a browser-like User-Agent
- Strips HTML tags, returns clean readable text
- Follows redirects (up to 3 hops)
- Returns: page title + text content
- On error: return the HTTP status code and error message

### Safety notes
- Only fetches public URLs; does not handle authentication
- Does not execute JavaScript (static HTML only)
- Respects robots.txt directives

### Example interactions
- "Read the Rust async book" → fetch the URL provided by user
- "What does this URL say: https://..." → fetch and summarize
- "Get the latest news from the Rust blog" → fetch blog URL

### Output format
Return the page title on the first line, then the text content. Summarize if the content is very long.
