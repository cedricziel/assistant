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

// Flutter SPA renders differently across runs — use a more generous ratio.
const MAX_DIFF_RATIO_FLUTTER = 0.15;

// Settle time for CSS transitions before screenshotting.
const CSS_SETTLE_MS = 300;

// Extra settle time for Flutter to render after network idle.
const FLUTTER_SETTLE_MS = 800;

// -- Helpers ----------------------------------------------------------------

/**
 * Authenticate for Flutter-rendered pages.
 *
 * Navigates to /setup with a `_token` query parameter. ConnectionScreen reads
 * this parameter in initState() and calls connect() automatically (GET /health
 * with the Bearer token). On success, flutter_secure_storage writes the
 * AES-GCM-encrypted profile and the router navigates to /chat.
 *
 * This avoids interacting with the Flutter canvas/semantic overlay, which
 * requires accessibility mode to be active and is unreliable in headless
 * Chromium without a screen reader.
 *
 * The router does not redirect /setup back to /setup (onSetup=true, no-op),
 * so the query parameter is still present in Uri.base when initState() runs.
 */
async function loginFlutter(page: Page) {
  // Navigate directly to the setup screen with the auto-connect token.
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "networkidle" });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);

  // Wait for GET /health to succeed and Flutter to navigate away from /setup.
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 15_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

/** Navigate and wait for network idle before screenshotting. */
async function navigateAndSettle(page: Page, path: string) {
  await page.goto(path, { waitUntil: "networkidle" });
  // Extra settle time for any CSS transitions and Flutter rendering.
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

// -- API helpers ------------------------------------------------------------

/** Make an authenticated JSON API request from within the browser context.
 *
 * Uses the Bearer token directly — the Flutter app uses Bearer auth, not
 * session cookies, so credentials: "same-origin" alone is not sufficient.
 */
async function apiGet(
  page: Page,
  path: string,
): Promise<{ status: number; body: unknown }> {
  return page.evaluate(
    async ([p, token]) => {
      const response = await fetch(p as string, {
        headers: { Authorization: `Bearer ${token}` },
      });
      let body: unknown = null;
      try {
        body = await response.json();
      } catch {
        body = null;
      }
      return { status: response.status, body };
    },
    [path, AUTH_TOKEN] as const,
  );
}

async function apiPost(
  page: Page,
  path: string,
  payload: unknown,
): Promise<{ status: number; body: unknown }> {
  return page.evaluate(
    async ([p, data, token]) => {
      const response = await fetch(p as string, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(data),
      });
      let body: unknown = null;
      try {
        body = await response.json();
      } catch {
        body = null;
      }
      return { status: response.status, body };
    },
    [path, payload, AUTH_TOKEN] as const,
  );
}

// -- Data creation helpers (API-based) --------------------------------------

/** Create a webhook via the REST API and return its id. */
async function createWebhook(page: Page): Promise<string> {
  const { body } = await apiPost(page, "/api/webhooks", {
    name: `E2E Webhook ${Date.now()}`,
    url: "https://example.com/webhook",
    event_types: ["turn.result"],
    active: true,
  });
  const id = (body as Record<string, unknown>)?.id as string | undefined;
  if (!id) {
    throw new Error(`Failed to create webhook: ${JSON.stringify(body)}`);
  }
  return id;
}

/** Create a workflow via the REST API and return its id. */
async function createWorkflow(page: Page): Promise<string> {
  const { body } = await apiPost(page, "/api/workflows", {
    name: `E2E Workflow ${Date.now()}`,
    description: "Visual regression test workflow",
    graph: {
      nodes: [
        { id: "trigger-1", kind: "trigger", config: { type: "manual" } },
        {
          id: "action-1",
          kind: "action",
          config: { type: "assistant_turn", prompt: "E2E test prompt" },
        },
      ],
      edges: [{ from: "trigger-1", to: "action-1" }],
    },
    active: false,
  });
  const id = (body as Record<string, unknown>)?.id as string | undefined;
  if (!id) {
    throw new Error(`Failed to create workflow: ${JSON.stringify(body)}`);
  }
  return id;
}

/** Create a user skill via the REST API and return its name. */
async function createSkill(page: Page): Promise<string> {
  const name = `e2e-skill-${Date.now()}`;
  const { body, status } = await apiPost(page, "/api/skills", {
    name,
    description: "Visual regression test skill",
    body: "# E2E Skill\n\nThis skill was created by the visual regression test suite.",
  });
  if (status !== 201) {
    throw new Error(`Failed to create skill: ${JSON.stringify(body)}`);
  }
  return name;
}

/** Create an agent via the REST API and return its id. */
async function createAgent(page: Page): Promise<string> {
  const { body } = await apiPost(page, "/api/agents", {
    card: {
      name: `E2E Agent ${Date.now()}`,
      description: "Visual regression test agent",
      version: "1.0.0",
      url: "https://example.com/agent",
      defaultInputModes: ["text"],
      defaultOutputModes: ["text"],
      capabilities: {
        streaming: false,
        pushNotifications: false,
        stateTransitionHistory: false,
      },
      supportedInterfaces: [],
      skills: [],
    },
    set_default: false,
  });
  const id = (body as Record<string, unknown>)?.id as string | undefined;
  if (!id) {
    throw new Error(`Failed to create agent: ${JSON.stringify(body)}`);
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
    await loginFlutter(page);
  });

  test("chat page", async ({ page }) => {
    await navigateAndSettle(page, "/chat");
    await expect(page).toHaveScreenshot("chat.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("traces page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/traces");
    await expect(page).toHaveScreenshot("traces-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("logs page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/logs");
    await expect(page).toHaveScreenshot("logs-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("skills list page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/skills");
    await expect(page).toHaveScreenshot("skills-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("contexts/personas list page", async ({ page }) => {
    await navigateAndSettle(page, "/contexts");
    await expect(page).toHaveScreenshot("contexts.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("analytics screen (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/analytics");
    await expect(page).toHaveScreenshot("analytics-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("agents list page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/agents");
    await expect(page).toHaveScreenshot("agents-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("webhooks list page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/webhooks");
    await expect(page).toHaveScreenshot("webhooks-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("workflows list page (empty state)", async ({ page }) => {
    await navigateAndSettle(page, "/workflows");
    await expect(page).toHaveScreenshot("workflows-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("persona create form", async ({ page }) => {
    await navigateAndSettle(page, "/contexts/new");
    await expect(page).toHaveScreenshot("persona-create-form.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("webhook detail screen", async ({ page }) => {
    const webhookId = await createWebhook(page);
    await navigateAndSettle(page, `/webhooks/${webhookId}`);
    await expect(page).toHaveScreenshot("webhook-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("workflow detail screen", async ({ page }) => {
    const workflowId = await createWorkflow(page);
    await navigateAndSettle(page, `/workflows/${workflowId}`);
    await expect(page).toHaveScreenshot("workflow-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("agent detail screen", async ({ page }) => {
    const agentId = await createAgent(page);
    await navigateAndSettle(page, `/agents/${agentId}`);
    await expect(page).toHaveScreenshot("agent-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("skill detail screen", async ({ page }) => {
    const skillName = await createSkill(page);
    await navigateAndSettle(page, `/skills/${skillName}`);
    await expect(page).toHaveScreenshot("skill-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("REST API: GET /api/personas returns JSON array", async ({ page }) => {
    const { status, body } = await apiGet(page, "/api/personas");
    expect(status, "GET /api/personas should return 200").toBe(200);
    expect(Array.isArray(body), "body should be a JSON array").toBeTruthy();
  });

  test("REST API: GET /api/conversations returns JSON array", async ({
    page,
  }) => {
    const { status, body } = await apiGet(page, "/api/conversations");
    expect(status, "GET /api/conversations should return 200").toBe(200);
    expect(Array.isArray(body), "body should be a JSON array").toBeTruthy();
  });

  test("REST API: POST /api/conversations creates a conversation", async ({
    page,
  }) => {
    const { status, body } = await apiPost(page, "/api/conversations", {
      title: "E2E test conversation",
    });
    expect(status, "POST /api/conversations should return 201").toBe(201);
    const b = body as Record<string, unknown>;
    expect(typeof b?.id, "response should include an id").toBe("string");
    expect(b?.title).toBe("E2E test conversation");
  });

  test("REST API: GET /api/traces returns JSON array", async ({ page }) => {
    const { status, body } = await apiGet(page, "/api/traces");
    expect(status, "GET /api/traces should return 200").toBe(200);
    expect(Array.isArray(body), "body should be a JSON array").toBeTruthy();
  });

  test("REST API: GET /api/logs returns JSON array", async ({ page }) => {
    const { status, body } = await apiGet(page, "/api/logs");
    expect(status, "GET /api/logs should return 200").toBe(200);
    expect(Array.isArray(body), "body should be a JSON array").toBeTruthy();
  });

  test("REST API: GET /api/personas/{id}/skills returns JSON array", async ({
    page,
  }) => {
    // Get the active persona id from the personas list
    const { status: listStatus, body: listBody } = await apiGet(
      page,
      "/api/personas",
    );
    expect(listStatus).toBe(200);
    const personas = listBody as Array<{ id: string }>;
    expect(
      personas.length,
      "at least one persona should exist",
    ).toBeGreaterThan(0);
    const personaId = personas[0].id;

    const { status, body } = await apiGet(
      page,
      `/api/personas/${personaId}/skills`,
    );
    expect(
      status,
      `GET /api/personas/${personaId}/skills should return 200`,
    ).toBe(200);
    expect(Array.isArray(body), "body should be a JSON array").toBeTruthy();
  });

  test("REST API: POST /api/personas/active switches active persona", async ({
    page,
  }) => {
    const { body: listBody } = await apiGet(page, "/api/personas");
    const personas = listBody as Array<{ id: string }>;
    expect(personas.length).toBeGreaterThan(0);

    const { status } = await apiPost(page, "/api/personas/active", {
      id: personas[0].id,
    });
    expect(status, "POST /api/personas/active should return 200").toBe(200);
  });

  test("REST API: GET /api/webhooks returns JSON array", async ({ page }) => {
    const { status, body } = await apiGet(page, "/api/webhooks");
    expect(status, "GET /api/webhooks should return 200").toBe(200);
    expect(Array.isArray(body), "body should be a JSON array").toBeTruthy();
  });

  test("REST API: GET /api/agents returns JSON array", async ({ page }) => {
    const { status, body } = await apiGet(page, "/api/agents");
    expect(status, "GET /api/agents should return 200").toBe(200);
    expect(Array.isArray(body), "body should be a JSON array").toBeTruthy();
  });

  test("REST API: GET /api/analytics returns summary object", async ({
    page,
  }) => {
    const { status, body } = await apiGet(page, "/api/analytics?window=24");
    expect(status, "GET /api/analytics should return 200").toBe(200);
    const b = body as Record<string, unknown>;
    expect(typeof b?.total_requests, "should have total_requests").toBe(
      "number",
    );
  });

  test("core routes avoid viewport horizontal overflow", async ({ page }) => {
    const routes = [
      "/chat",
      "/traces",
      "/logs",
      "/analytics",
      "/agents",
      "/webhooks",
      "/workflows",
      "/contexts",
      "/contexts/new",
      "/skills",
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
