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

/** Create an agent through the UI and return its id. */
async function createAgent(page: Page): Promise<string> {
  await navigateAndSettle(page, "/agents/new");

  const suffix = Date.now();
  await page.fill('input[name="name"]', `E2E Agent ${suffix}`);
  await page.fill(
    'textarea[name="description"]',
    "Agent created by visual regression test",
  );
  await page.fill('input[name="version"]', "1.0.0");

  await page.locator("form.entity-form button[type='submit']").click();

  await page.waitForURL(/\/agents\/[^/]+$/);
  const pathname = new URL(page.url()).pathname;
  const id = pathname.split("/").pop();
  if (!id) {
    throw new Error(`failed to parse agent id from ${pathname}`);
  }
  return id;
}

/** Create a user skill through the UI and return its name. */
async function createSkill(
  page: Page,
  suffix: string = String(Date.now()),
): Promise<string> {
  await navigateAndSettle(page, "/skills/new");

  const name = `e2e-skill-${suffix}`;
  await page.fill('input[name="name"]', name);
  await page.fill('input[name="description"]', "E2E test skill");
  await page.fill('textarea[name="body"]', "When asked, do the e2e thing.");

  await page.locator('form button[type="submit"]').click();
  await page.waitForURL(/\/skills\/[^/]+$/);
  return name;
}

/** Create a webhook through the UI and return its id. */
async function createWebhook(page: Page): Promise<string> {
  await navigateAndSettle(page, "/webhooks/new");

  const suffix = Date.now();
  await page.fill('input[name="name"]', `E2E Webhook ${suffix}`);
  await page.fill('input[name="url"]', `https://example.com/e2e-${suffix}`);
  await page.fill('input[name="event_types"]', "turn.result, tool.result");

  await page.locator("form.entity-form button[type='submit']").click();

  await page.waitForURL(/\/webhooks\/[^/]+$/);
  const pathname = new URL(page.url()).pathname;
  const id = pathname.split("/").pop();
  if (!id) {
    throw new Error(`failed to parse webhook id from ${pathname}`);
  }
  return id;
}

/** Queue a workflow run and return the run id. */
async function queueWorkflowRun(
  page: Page,
  workflowId: string,
): Promise<string> {
  const payload = (await page.evaluate(async (id) => {
    const response = await fetch(`/api/workflows/${id}/test-run`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({}),
      credentials: "same-origin",
    });

    let body: unknown = null;
    try {
      body = await response.json();
    } catch {
      body = null;
    }

    return {
      ok: response.ok,
      status: response.status,
      body,
    };
  }, workflowId)) as {
    ok: boolean;
    status: number;
    body: { run_id?: string } | null;
  };

  expect(
    payload.ok,
    `workflow test-run request failed (${payload.status})`,
  ).toBeTruthy();
  const runId = payload.body?.run_id;
  if (!runId) {
    throw new Error("failed to parse workflow run id from test-run response");
  }
  return runId;
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

  test("agent detail and edit screens", async ({ page }) => {
    const agentId = await createAgent(page);

    await navigateAndSettle(page, `/agents/${agentId}`);
    await expect(page).toHaveScreenshot("agent-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });

    await navigateAndSettle(page, `/agents/${agentId}/edit`);
    await expect(page).toHaveScreenshot("agent-edit.png", {
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

  test("webhook detail and edit screens", async ({ page }) => {
    const webhookId = await createWebhook(page);

    await navigateAndSettle(page, `/webhooks/${webhookId}`);
    await expect(page).toHaveScreenshot("webhook-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
      mask: [
        page.locator(".hdr-trace-id"),
        page.locator(".entity-section").filter({ hasText: "Endpoint URL" }),
        page.locator("#secret-value"),
        page.locator(".entity-section").filter({ hasText: "Timestamps" }),
      ],
    });

    await navigateAndSettle(page, `/webhooks/${webhookId}/edit`);
    await expect(page).toHaveScreenshot("webhook-edit.png", {
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
      mask: [page.locator("table tbody tr")],
    });

    await navigateAndSettle(page, `/workflows/${workflowId}`);

    await expect(page).toHaveScreenshot("workflow-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });

    await page
      .locator(".workflow-detail-page [data-mode='runs']")
      .first()
      .click();
    await page.waitForTimeout(CSS_SETTLE_MS);
    await expect(page).toHaveScreenshot("workflow-detail-runs.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });

    const viewport = page.viewportSize();
    const width = viewport ? viewport.width : 1280;
    if (width >= 768) {
      await page
        .locator(".workflow-detail-page [data-mode='graph']")
        .first()
        .click();
      await page.waitForTimeout(CSS_SETTLE_MS);
      await expect(page).toHaveScreenshot("workflow-detail-graph.png", {
        fullPage: true,
        maxDiffPixelRatio: MAX_DIFF_RATIO,
      });
    }

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

    await page
      .locator(".workflow-editor-page [data-mode='steps']")
      .first()
      .click();
    await page.waitForTimeout(CSS_SETTLE_MS);
    await expect(page).toHaveScreenshot("workflow-editor-steps.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });

    if (width >= 768) {
      await page
        .locator(".workflow-editor-page [data-mode='graph']")
        .first()
        .click();
      await page.waitForTimeout(CSS_SETTLE_MS);
      await expect(page).toHaveScreenshot("workflow-editor-graph.png", {
        fullPage: true,
        maxDiffPixelRatio: MAX_DIFF_RATIO,
      });
    }

    const runId = await queueWorkflowRun(page, workflowId);
    await navigateAndSettle(page, `/workflows/${workflowId}/runs/${runId}`);
    await expect(page).toHaveScreenshot("workflow-run-detail.png", {
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

  test("contexts page", async ({ page }) => {
    await navigateAndSettle(page, "/contexts");
    await expect(page).toHaveScreenshot("contexts.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("skills list page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/skills");
    await expect(page).toHaveScreenshot("skills-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("skill create form", async ({ page }) => {
    await navigateAndSettle(page, "/skills/new");
    await expect(page).toHaveScreenshot("skill-form.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("skill detail and edit screens", async ({ page }) => {
    const skillName = await createSkill(page);

    await navigateAndSettle(page, `/skills/${skillName}`);
    await expect(page).toHaveScreenshot("skill-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });

    await navigateAndSettle(page, `/skills/${skillName}/edit`);
    await expect(page).toHaveScreenshot("skill-edit.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("skill CRUD flow", async ({ page }) => {
    const suffix = String(Date.now());
    const name = `e2e-skill-${suffix}`;

    // Create
    await navigateAndSettle(page, "/skills/new");
    await page.fill('input[name="name"]', name);
    await page.fill('input[name="description"]', "CRUD flow test skill");
    await page.fill('textarea[name="body"]', "When asked, do the crud thing.");
    await page.locator('form button[type="submit"]').click();
    await page.waitForURL(/\/skills\/[^/]+$/);
    expect(new URL(page.url()).pathname).toContain(name);

    // Edit
    await navigateAndSettle(page, `/skills/${name}/edit`);
    await page.fill('input[name="description"]', "Updated description");
    await page.locator('form button[type="submit"]').click();
    await page.waitForURL(/\/skills\/[^/]+$/);

    // Verify updated description visible on detail page
    await navigateAndSettle(page, `/skills/${name}`);
    const bodyText = await page.textContent("body");
    expect(bodyText).toContain("Updated description");

    // Delete
    await page.locator('form[action*="/delete"] button[type="submit"]').click();
    await page.waitForURL("/skills");
    const listText = await page.textContent("body");
    expect(listText).not.toContain(name);
  });

  test("persona skill access page", async ({ page }) => {
    // Navigate to the default persona's skill access page
    // First find a persona id by going to contexts page
    await navigateAndSettle(page, "/contexts");
    const firstPersonaLink = page.locator('a[href*="/personas/"]').first();
    const personaHref = await firstPersonaLink.getAttribute("href");
    if (!personaHref) {
      // No personas yet — skip screenshot, just verify no crash
      return;
    }
    const personaId = personaHref.split("/personas/")[1]?.split("/")[0];
    if (!personaId) return;

    await navigateAndSettle(page, `/personas/${personaId}/skills`);
    await expect(page).toHaveScreenshot("persona-skill-access.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO,
    });
  });

  test("responsive navigation switches by breakpoint", async ({ page }) => {
    await navigateAndSettle(page, "/chat");
    const viewport = page.viewportSize();
    const width = viewport ? viewport.width : 1280;

    const iconRailVisible = await isVisible(page, ".icon-rail");
    const topBarVisible = await isVisible(page, ".top-bar");
    const bottomTabsVisible = await isVisible(page, ".bottom-tabs");
    const hamburgerVisible = await isVisible(page, ".hamburger");

    if (width <= 640) {
      expect(bottomTabsVisible).toBeTruthy();
      expect(topBarVisible).toBeFalsy();
      expect(iconRailVisible).toBeFalsy();
      expect(hamburgerVisible).toBeFalsy();
      return;
    }

    if (width <= 900) {
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

  test("mobile more tab points to active management section", async ({
    page,
  }) => {
    const cases: Array<{ path: string; expectedHref: string }> = [
      { path: "/agents", expectedHref: "/agents" },
      { path: "/workflows", expectedHref: "/workflows" },
      { path: "/webhooks", expectedHref: "/webhooks" },
      { path: "/contexts", expectedHref: "/contexts" },
    ];

    for (const item of cases) {
      await navigateAndSettle(page, item.path);

      const href = await page
        .locator(".bottom-tabs .tab-item")
        .nth(4)
        .getAttribute("href");
      expect(href, `more tab href on ${item.path}`).toBe(item.expectedHref);

      const ariaCurrent = await page
        .locator(".bottom-tabs .tab-item")
        .nth(4)
        .getAttribute("aria-current");
      expect(ariaCurrent, `more tab aria-current on ${item.path}`).toBe("page");
    }
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
      "/skills",
      "/skills/new",
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
