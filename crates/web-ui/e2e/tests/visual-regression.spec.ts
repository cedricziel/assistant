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

// Cross-platform font rendering (macOS vs Linux CI) causes up to ~4% pixel diffs.
// 5% tolerance absorbs font hinting differences while still catching layout regressions.
const MAX_DIFF_RATIO = 0.05;

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

/** Check whether an element currently takes up layout space. */
async function isVisible(page: Page, selector: string): Promise<boolean> {
  return page.evaluate((sel) => {
    const el = document.querySelector(sel) as HTMLElement | null;
    if (!el) return false;
    return !!(el.offsetWidth || el.offsetHeight || el.getClientRects().length);
  }, selector);
}

/** Create a workflow through the UI and return its id. */
async function createWorkflow(page: Page): Promise<string> {
  await navigateAndSettle(page, "/workflows/new");

  const suffix = Date.now();
  await page.fill('input[name="name"]', `E2E Workflow ${suffix}`);
  await page.fill(
    'input[name="description"]',
    "Workflow created by visual regression test",
  );

  await page.locator("form.wf-form button[type='submit']").click();

  await page.waitForURL(/\/workflows\/[^/]+$/);
  const pathname = new URL(page.url()).pathname;
  const id = pathname.split("/").pop();
  if (!id) {
    throw new Error(`failed to parse workflow id from ${pathname}`);
  }
  return id;
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

  test("workflow create form", async ({ page }) => {
    await navigateAndSettle(page, "/workflows/new");
    await expect(page).toHaveScreenshot("workflow-form.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("workflow detail/edit/editor sub-screens", async ({ page }) => {
    const workflowId = await createWorkflow(page);

    await navigateAndSettle(page, "/workflows");
    await expect(page).toHaveScreenshot("workflow-list.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });

    await navigateAndSettle(page, `/workflows/${workflowId}`);

    await expect(page).toHaveScreenshot("workflow-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });

    await navigateAndSettle(page, `/workflows/${workflowId}/edit`);
    await expect(page).toHaveScreenshot("workflow-edit.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });

    await navigateAndSettle(page, `/workflows/${workflowId}/editor`);
    await expect(page).toHaveScreenshot("workflow-editor.png", {
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

  test("contexts page loads and shows key UI", async ({ page }) => {
    await navigateAndSettle(page, "/contexts");
    await expect(
      page.getByRole("heading", { name: "Assistant Contexts" }),
    ).toBeVisible();
    await expect(page.locator("#main-content")).toBeVisible();
  });

  test("responsive navigation switches by breakpoint", async ({ page }) => {
    await navigateAndSettle(page, "/chat");
    const viewport = page.viewportSize();
    const width = viewport ? viewport.width : 1280;

    const iconRailVisible = await isVisible(page, ".icon-rail");
    const topBarVisible = await isVisible(page, ".top-bar");
    const bottomTabsVisible = await isVisible(page, ".bottom-tabs");
    const hamburgerVisible = await isVisible(page, ".hamburger");

    if (width < 640) {
      expect(bottomTabsVisible).toBeTruthy();
      expect(topBarVisible).toBeFalsy();
      expect(iconRailVisible).toBeFalsy();
      expect(hamburgerVisible).toBeFalsy();
      return;
    }

    if (width < 900) {
      expect(bottomTabsVisible).toBeFalsy();
      expect(topBarVisible).toBeTruthy();
      expect(iconRailVisible).toBeFalsy();
      expect(hamburgerVisible).toBeTruthy();
      return;
    }

    expect(bottomTabsVisible).toBeFalsy();
    expect(topBarVisible).toBeTruthy();
    expect(iconRailVisible).toBeTruthy();
  });

  test("core routes avoid viewport horizontal overflow", async ({ page }) => {
    const routes = [
      "/chat",
      "/traces",
      "/logs",
      "/analytics",
      "/agents",
      "/agents/new",
      "/webhooks",
      "/webhooks/new",
      "/workflows",
      "/workflows/new",
      "/contexts",
    ];

    for (const route of routes) {
      await navigateAndSettle(page, route);
      const hasOverflow = await page.evaluate(() => {
        const root = document.documentElement;
        return root.scrollWidth > root.clientWidth + 1;
      });
      expect(hasOverflow, `viewport overflow on ${route}`).toBeFalsy();
    }
  });
});
