---
name: e2e-testing
description: >
  Playwright visual regression testing for the assistant web UI. Covers
  test structure, screenshot baselines, cross-platform diff tolerance,
  CI workflow with inline diff comments, and baseline management.
  Use when adding pages, changing layouts, or debugging visual test failures.
license: MIT
---

# E2E Testing (Playwright Visual Regression)

The assistant web UI uses Playwright for screenshot-based visual regression
testing. Every page is captured at three viewport sizes (desktop, tablet,
mobile) and compared against committed baselines.

## Directory Layout

```text
crates/web-ui/e2e/
  playwright.config.ts     # Config: viewports, server, reporters
  package.json             # Dependencies (playwright, @playwright/test)
  tests/
    visual-regression.spec.ts   # All visual tests
  screenshots/                  # Committed baselines (PNG)
    tests/visual-regression.spec.ts/
      login-desktop-chrome.png
      login-tablet-chrome.png
      login-mobile-chrome.png
      ...
  test-results/            # Generated on failure (diff, actual, expected PNGs)
  playwright-report/       # HTML report (gitignored)
```

## Running Tests

```bash
# From the e2e directory
cd crates/web-ui/e2e

# Run all visual tests (starts server automatically via webServer config)
npx playwright test

# Update baselines after intentional changes
npx playwright test --update-snapshots
# or use the npm script:
npm run test:update

# Run a single test
npx playwright test -g "traces page"

# Run only desktop viewport
npx playwright test --project=desktop-chrome

# Show HTML report after failure
npx playwright show-report
```

The `webServer` config in `playwright.config.ts` automatically builds and
starts the web-ui binary with `--auth-token test-token --listen 127.0.0.1:8787`.
Set `E2E_BASE_URL` to skip the auto-start and use a running server instead.

## Writing Tests

### Test Structure

```typescript
import { test, expect, Page } from "@playwright/test";

const AUTH_TOKEN = "test-token";
const MAX_DIFF_RATIO = 0.03; // 3% tolerance for cross-platform fonts
const CSS_SETTLE_MS = 300; // Wait for CSS transitions

// Authenticate via the login form
async function login(page: Page) {
  await page.goto("/login");
  await page.fill('input[name="token"]', AUTH_TOKEN);
  await page.click('button[type="submit"]');
  await page.waitForURL((url) => !url.pathname.includes("/login"));
}

// Navigate and wait for network idle + CSS settle
async function navigateAndSettle(page: Page, path: string) {
  await page.goto(path, { waitUntil: "networkidle" });
  await page.waitForTimeout(CSS_SETTLE_MS);
}
```

### Adding a New Page Test

```typescript
test("my new page (empty state)", async ({ page }) => {
  await navigateAndSettle(page, "/my-page");
  await expect(page).toHaveScreenshot("my-page-empty.png", {
    fullPage: true,
    maxDiffPixelRatio: MAX_DIFF_RATIO,
  });
});
```

Then generate baselines:

```bash
npx playwright test --update-snapshots -g "my new page"
```

This creates three files in `screenshots/` (one per project/viewport).

### Unauthenticated vs Authenticated Pages

- **Login page:** Test without calling `login()` first
- **All other pages:** Use `test.beforeEach` with `login(page)`:

```typescript
test.describe("Authenticated pages", () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test("page name", async ({ page }) => { ... });
});
```

## Cross-Platform Tolerance

Font rendering differs between macOS (local dev) and Linux (CI). The
`maxDiffPixelRatio: 0.03` setting allows up to 3% pixel differences,
which absorbs font hinting/anti-aliasing variance while still catching
layout regressions (moved elements, missing sections, broken styles).

**When to adjust:**

- If CI fails on font-only diffs, the 3% tolerance should already absorb them
- If a real layout regression is masked, _lower_ the tolerance for that specific test
- Never set tolerance above 5% — at that point you're not testing anything

## Viewport Projects

Three projects in `playwright.config.ts` match the app's responsive breakpoints:

| Project          | Viewport | App Layout                    |
| ---------------- | -------- | ----------------------------- |
| `desktop-chrome` | 1280x900 | Icon rail + top bar           |
| `tablet-chrome`  | 768x1024 | Hamburger + drawer            |
| `mobile-chrome`  | Pixel 7  | Bottom tabs + stacked content |

## CI Workflow

The `visual-regression` job in `.github/workflows/ci.yml`:

1. Builds the web-ui binary
2. Installs Playwright + Chromium
3. Runs `npx playwright test`
4. Uploads the HTML report as an artifact (always)
5. On failure + PR: uploads diff images as artifact
6. On failure + PR: pushes diff PNGs to an orphan `visual-diffs/pr-N` branch
   and posts an inline comment with embedded image comparisons
7. On PR close: a cleanup job deletes the `visual-diffs/pr-N` branch

### Reading Diff Comments

When visual tests fail on a PR, the bot posts a comment with:

- **Expected:** The committed baseline
- **Actual:** What the test rendered
- **Diff:** Pink/red highlights showing changed pixels

Review the diff images to decide whether to:

- **Fix a regression:** The change was unintentional — fix the code
- **Update baselines:** The change was intentional — run `npm run test:update` and commit

### Regenerating Baselines

After intentional visual changes:

```bash
cd crates/web-ui/e2e
npx playwright test --update-snapshots
```

Commit the updated PNGs in the same commit as the code change.

## Troubleshooting

### Tests pass locally but fail in CI

Font rendering differences. The 3% tolerance should absorb this. If not:

1. Check if the diff is font-only (fuzzy text edges) vs structural (moved elements)
2. For font-only diffs: consider bumping tolerance for that specific test
3. For structural diffs: there's a real bug — investigate

### Tests fail after adding `app_css_url` to a template

The CSS is identical whether inline or external. If tests fail, ensure:

1. The `static_assets` router is mounted _before_ the auth middleware
2. The fingerprinted URL is being served correctly (check `/static/app.css`)

### Server doesn't start in time

The `webServer.timeout` is 120 seconds (enough for a fresh `cargo build`).
If it still times out:

1. Pre-build: `cargo build -p assistant-web-ui` before running tests
2. Check if port 8787 is already in use
3. Set `E2E_BASE_URL` to point to a manually started server

### Screenshot dimensions changed

Viewport size is fixed per project — if dimensions change, it's likely
the page content height changed. `fullPage: true` captures the full
scrollable height, so adding content to a page will change the baseline.
