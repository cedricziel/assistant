import { test, expect, Page } from "@playwright/test";

/**
 * Visual regression tests for the assistant web UI.
 *
 * Each test navigates to a page and captures a full-page screenshot that is
 * compared against a committed baseline in `screenshots/`.
 *
 * Run `npm run test:update` to regenerate baselines after intentional changes.
 */

const AUTH_TOKEN = "test-token";

// Cross-platform font rendering (macOS vs Linux CI) causes ~2% pixel diffs.
// 3% tolerance absorbs font hinting differences while still catching layout regressions.
const MAX_DIFF_RATIO = 0.03;

// Settle time for CSS transitions before screenshotting.
const CSS_SETTLE_MS = 300;

// -- Helpers ----------------------------------------------------------------

/** Authenticate by submitting the login form. */
async function login(page: Page) {
  await page.goto("/login");
  await page.fill('input[name="token"]', AUTH_TOKEN);
  await page.click('button[type="submit"]');
  // Wait for redirect to complete
  await page.waitForURL((url) => !url.pathname.includes("/login"));
}

/** Navigate and wait for network idle before screenshotting. */
async function navigateAndSettle(page: Page, path: string) {
  await page.goto(path, { waitUntil: "networkidle" });
  // Extra settle time for any CSS transitions
  await page.waitForTimeout(CSS_SETTLE_MS);
}

// -- Tests ------------------------------------------------------------------

test.describe("Login page", () => {
  test("login page renders correctly", async ({ page }) => {
    await navigateAndSettle(page, "/login");
    await expect(page).toHaveScreenshot("login.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("login page shows error on invalid token", async ({ page }) => {
    await page.goto("/login");
    await page.fill('input[name="token"]', "wrong-token");
    await page.click('button[type="submit"]');
    await page.waitForSelector(".login-error");
    await page.waitForTimeout(CSS_SETTLE_MS);
    await expect(page).toHaveScreenshot("login-error.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });
});

test.describe("Authenticated pages", () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test("traces page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/traces");
    await expect(page).toHaveScreenshot("traces-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("logs page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/logs");
    await expect(page).toHaveScreenshot("logs-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("analytics page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/analytics");
    await expect(page).toHaveScreenshot("analytics-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("agents list page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/agents");
    await expect(page).toHaveScreenshot("agents-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("agent create form", async ({ page }) => {
    await navigateAndSettle(page, "/agents/new");
    await expect(page).toHaveScreenshot("agent-form.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("webhooks list page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/webhooks");
    await expect(page).toHaveScreenshot("webhooks-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("webhook create form", async ({ page }) => {
    await navigateAndSettle(page, "/webhooks/new");
    await expect(page).toHaveScreenshot("webhook-form.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("chat page", async ({ page }) => {
    await navigateAndSettle(page, "/chat");
    await expect(page).toHaveScreenshot("chat.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });
});
