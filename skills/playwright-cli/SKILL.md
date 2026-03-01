---
name: playwright-cli
description: >
  Browser automation via @playwright/mcp (Microsoft). Use this when the user
  wants to navigate websites, fill forms, take screenshots, scrape web content,
  test web apps, or run any multi-step browser workflow. Requires no display
  (headless mode supported).
license: MIT
compatibility: "Requires: npx (@playwright/mcp)"
---

# playwright-cli

Browser automation via `@playwright/mcp` (Microsoft). Navigate websites,
fill forms, take screenshots, and run complex web automation workflows.

## Setup

`@playwright/mcp` is available via npx. No global install required.

## When to use this skill

- Automate website interactions (clicking, scrolling, forms)
- Take screenshots or generate PDFs of web pages
- Web scraping when no API is available
- Test web applications
- Multi-step workflows on websites

## Execution

Start the MCP server:

```bash
npx @playwright/mcp@latest --headless
```

### Key flags

- `--headless` — No visible browser window (recommended on servers)
- `--browser chromium` — Select browser: chromium / firefox / webkit
- `--allowed-origins https://example.com` — Restrict access to specific hosts
- `--port 3000` — Port for the MCP server

### Browser agent capabilities

- **Navigate:** Open URL, back/forward, reload
- **Interact:** Click, double-click, hover, drag & drop, type, checkboxes, dropdowns
- **Read:** Text, elements, screenshots (PNG), PDFs
- **Tabs:** Open, switch, close
- **Sessions:** Save storage state (cookies, localStorage) for repeated logins
- **Request mocking:** Intercept and manipulate HTTP requests

## Security notes

- Use `--allowed-origins` to restrict access where possible
- Use `--headless` on servers — no display required
- Storage state files contain cookies/tokens — store securely

## Example invocations

**Headless browser session (chromium):**

```bash
npx @playwright/mcp@latest --headless --browser chromium
```

**Restricted origin:**

```bash
npx @playwright/mcp@latest --headless --allowed-origins https://example.com
```

**With visible browser (local dev):**

```bash
npx @playwright/mcp@latest --browser chromium
```
