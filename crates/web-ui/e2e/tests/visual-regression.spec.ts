import { test, expect, Page, Route } from "@playwright/test";

import { enableAccessibility } from "./support/flutter";
import { mockEverything } from "./support/integration-mocks";

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
// Flutter web needs time to: initialize WASM, load async providers from
// SharedPreferences, re-evaluate the router, and render the final state.
// 3 seconds is the observed upper bound on a developer MacBook; CI may need more.
const FLUTTER_SETTLE_MS = 3000;

// -- Helpers ----------------------------------------------------------------

/**
 * Authenticate for Flutter-rendered pages.
 *
 * Navigates to /setup with a `_token` query parameter. ConnectionScreen reads
 * this parameter in initState() and calls connect() automatically (GET /health
 * with the Bearer token). On success, flutter_secure_storage writes the
 * context and the router navigates to /chat.
 *
 * Uses `waitUntil: "load"` instead of `"networkidle"` because the Flutter
 * WASM engine makes continuous network requests during initialisation, which
 * can prevent `networkidle` from ever firing within the test timeout.
 */
async function loginFlutter(page: Page) {
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "load" });
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 20_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

/** Navigate and wait for the page to settle before screenshotting.
 *
 * Uses `waitUntil: "load"` because Flutter WASM keeps network connections
 * open (SSE streams, polling) which prevents `"networkidle"` from firing.
 */
async function navigateAndSettle(page: Page, path: string) {
  await page.goto(path, { waitUntil: "load" });
  // Extra settle time for Flutter WASM init + rendering.
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

/**
 * Stub `GET /api/integrations/catalog` so the screenshot does not depend on the
 * live integrations.sh catalog that the test server syncs at startup.
 */
async function mockCatalog(page: Page, items: unknown[]) {
  await page.route(/\/api\/integrations\/catalog(\?.*)?$/, (route) =>
    route.fulfill({ json: { items } }),
  );
}

const CATALOG_FIXTURE = [
  {
    id: "curated/linear-app-mcp",
    name: "Linear",
    description: "Issue tracking and project planning for product teams.",
    domain: "linear.app",
    categories: ["project-management", "dev"],
    connect_url: "https://mcp.linear.app/mcp",
    auth: { kind: "oauth" },
    installable_layers: ["org", "space", "user"],
  },
  {
    id: "curated/github-mcp",
    name: "GitHub",
    description: "Repositories, pull requests and issues.",
    domain: "github.com",
    categories: ["dev"],
    connect_url: "https://api.githubcopilot.com/mcp/",
    auth: {
      kind: "header",
      header_name: "Authorization",
      placeholder: "pat",
      value_prefix: "Bearer ",
    },
  },
  {
    id: "curated/deepwiki-mcp",
    name: "DeepWiki",
    description: "Documentation search for public repositories.",
    domain: "deepwiki.com",
    categories: ["docs"],
    connect_url: "https://mcp.deepwiki.com/mcp",
    auth: { kind: "none" },
  },
];

const SETTINGS_ME = "user-1";

/**
 * Mock a space admin opening the settings screen: identity, org, spaces,
 * membership, the org's users (for owner labels) and the two rule scopes.
 *
 * The `scope=space` response adds two members' rules, so the screenshot shows
 * the "Members' approvals" group with a known member name and a bare user id.
 */
async function mockSettingsAdmin(page: Page) {
  const selection = {
    orgId: "org-1",
    orgName: "Acme",
    spaceId: "eng",
    spaceName: "Engineering",
  };
  // The persistent context runs login before this test body, so seed the
  // selection now; the next full navigation reloads the app with it.
  await page.evaluate(
    (value) =>
      window.localStorage.setItem(
        "assistant.spaceSelection",
        JSON.stringify(value),
      ),
    selection,
  );
  await page.addInitScript((value) => {
    window.localStorage.setItem(
      "assistant.spaceSelection",
      JSON.stringify(value),
    );
  }, selection);

  const json = (route: Route, body: unknown) => route.fulfill({ json: body });

  await page.route(/\/api\/users\/me$/, (route) =>
    json(route, {
      id: SETTINGS_ME,
      email: "ada@example.com",
      name: "Ada",
      org_id: "org-1",
      created_at: "2026-01-01T09:00:00Z",
      updated_at: "2026-01-01T09:00:00Z",
    }),
  );
  await page.route(/\/api\/orgs$/, (route) =>
    json(route, [
      { id: "org-1", name: "Acme", slug: "acme", auth_mode: "password" },
    ]),
  );
  await page.route(/\/api\/orgs\/org-1$/, (route) =>
    json(route, {
      id: "org-1",
      name: "Acme",
      slug: "acme",
      auth_mode: "password",
      created_at: "2026-01-01T09:00:00Z",
      updated_at: "2026-01-01T09:00:00Z",
    }),
  );
  await page.route(/\/api\/orgs\/org-1\/spaces$/, (route) =>
    json(route, [{ id: "eng", name: "Engineering", slug: "eng" }]),
  );
  await page.route(/\/api\/orgs\/org-1\/spaces\/eng\/members$/, (route) =>
    json(route, [
      {
        space_id: "eng",
        user_id: SETTINGS_ME,
        role: "space_admin",
        created_at: "2026-01-01T09:00:00Z",
      },
    ]),
  );
  await page.route(/\/api\/orgs\/org-1\/users$/, (route) =>
    json(route, [
      {
        id: SETTINGS_ME,
        email: "ada@example.com",
        name: "Ada",
        created_at: "2026-01-01T09:00:00Z",
      },
      {
        id: "user-2",
        email: "grace@example.com",
        name: "Grace",
        created_at: "2026-01-01T09:00:00Z",
      },
    ]),
  );
  await page.route(/\/api\/tool-permission-rules(\?.*)?$/, (route) => {
    const scope = new URL(route.request().url()).searchParams.get("scope");
    return json(
      route,
      scope === "space"
        ? TOOL_PERMISSION_RULES_SPACE_FIXTURE
        : TOOL_PERMISSION_RULES_FIXTURE,
    );
  });
}

const TOOL_PERMISSION_RULES_FIXTURE = [
  {
    id: "rule-1",
    user_id: "user-1",
    effect: "allow",
    rule: "bash(git status*)",
    tool_name: "bash",
    created_at: "2026-01-02T09:00:00Z",
  },
  {
    id: "rule-2",
    user_id: "user-1",
    effect: "allow",
    rule: "web-fetch(https://docs.rs/*)",
    tool_name: "web-fetch",
    created_at: "2026-01-03T09:00:00Z",
  },
  {
    id: "rule-3",
    user_id: "user-1",
    effect: "deny",
    rule: "bash(rm *)",
    tool_name: "bash",
    created_at: "2026-01-04T09:00:00Z",
  },
];

/** The same rules plus two members', as returned for `scope=space`. */
const TOOL_PERMISSION_RULES_SPACE_FIXTURE = [
  ...TOOL_PERMISSION_RULES_FIXTURE,
  {
    id: "rule-4",
    user_id: "user-2",
    effect: "allow",
    rule: "bash(npm test*)",
    tool_name: "bash",
    created_at: "2026-01-05T09:00:00Z",
  },
  {
    id: "rule-5",
    user_id: "user-3",
    effect: "deny",
    rule: "web-fetch(https://*.example.com/*)",
    tool_name: "web-fetch",
    created_at: "2026-01-06T09:00:00Z",
  },
];

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

/**
 * Create a workflow with a fan-in edge pattern (complex DAG) via the REST API.
 * The mobile editor detects this and shows the "Complex graph" banner.
 */
async function createComplexWorkflow(page: Page): Promise<string> {
  const { body } = await apiPost(page, "/api/workflows", {
    name: `E2E Complex Workflow ${Date.now()}`,
    description: "Fan-in DAG for visual regression",
    graph: {
      version: 1,
      nodes: [
        { id: "t1", kind: "trigger", config: { type: "manual" } },
        {
          id: "a1",
          kind: "action",
          config: {
            type: "http_request",
            url: "https://a.example.com",
            method: "GET",
          },
        },
        {
          id: "a2",
          kind: "action",
          config: {
            type: "http_request",
            url: "https://b.example.com",
            method: "POST",
          },
        },
      ],
      // t1→a1, t1→a2, a1→a2 gives a2 two incoming edges (fan-in)
      edges: [
        { from: "t1", to: "a1", on: "trigger" },
        { from: "t1", to: "a2", on: "trigger" },
        { from: "a1", to: "a2", on: "success" },
      ],
      execution: { max_steps: 200, max_visits_per_node: 25 },
    },
    active: false,
  });
  const id = (body as Record<string, unknown>)?.id as string | undefined;
  if (!id) {
    throw new Error(
      `Failed to create complex workflow: ${JSON.stringify(body)}`,
    );
  }
  return id;
}

/**
 * Create a linear 3-node workflow (trigger + 2 actions) for mobile editor
 * baseline tests.
 */
async function createLinearWorkflow(page: Page): Promise<string> {
  const { body } = await apiPost(page, "/api/workflows", {
    name: `E2E Linear Workflow ${Date.now()}`,
    description: "Linear chain for mobile editor baseline",
    graph: {
      version: 1,
      nodes: [
        { id: "t1", kind: "trigger", config: { type: "manual" } },
        {
          id: "a1",
          kind: "action",
          config: {
            type: "http_request",
            url: "https://step1.example.com",
            method: "GET",
          },
        },
        {
          id: "a2",
          kind: "action",
          config: { type: "assistant_turn", prompt: "Summarise the response" },
        },
      ],
      edges: [
        { from: "t1", to: "a1", on: "trigger" },
        { from: "a1", to: "a2", on: "success" },
      ],
      execution: { max_steps: 200, max_visits_per_node: 25 },
    },
    active: false,
  });
  const id = (body as Record<string, unknown>)?.id as string | undefined;
  if (!id) {
    throw new Error(
      `Failed to create linear workflow: ${JSON.stringify(body)}`,
    );
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

/** Create a persona via the REST API and return its id. */
async function createPersona(page: Page): Promise<string> {
  const id = `e2e-persona-${Date.now()}`;
  const { body, status } = await apiPost(page, "/api/personas", {
    id,
    name: `E2E Persona ${Date.now()}`,
  });
  if (status !== 201) {
    throw new Error(`Failed to create persona: ${JSON.stringify(body)}`);
  }
  return id;
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
  // The Flutter web app serves a token-only login screen at /login.
  // Unauthenticated visitors are redirected here automatically.
  test("login page renders correctly", async ({ page }) => {
    await navigateAndSettle(page, "/login");
    await expect(page).toHaveScreenshot("login.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
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

  test("chat page with Persona starter prompts", async ({ page }) => {
    // The empty chat shows the active Persona's starters. Patch the real
    // Persona detail response instead of storing prompts on the seeded Persona.
    await page.route(/\/api\/personas\/[^/?]+(\?.*)?$/, async (route) => {
      if (route.request().method() !== "GET") return route.fallback();
      const response = await route.fetch();
      const body = await response.json().catch(() => null);
      if (!response.ok() || body === null || typeof body.id !== "string") {
        return route.fulfill({ response });
      }
      return route.fulfill({
        response,
        json: {
          ...body,
          starter_prompts: [
            "Plan my week",
            "Summarize my inbox",
            "Draft a status update",
            "Explain what you can do",
          ],
          can_edit_starter_prompts: true,
        },
      });
    });
    await navigateAndSettle(page, "/chat");
    await expect(page).toHaveScreenshot("empty-chat-starters.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("chat page with follow-up suggestion chips", async ({ page }) => {
    // The e2e server's LLM is a scripted stand-in, so fixture the turn: the
    // POST stream ends at `done`, and the run's replay tail carries the
    // `suggestions` frame the app listens for afterwards.
    const runId = "00000000-0000-4000-8000-0000000000a1";
    const messageId = "00000000-0000-4000-8000-0000000000a2";
    const sse = (frames: [string, string, unknown][]) =>
      frames
        .map(
          ([event, id, data]) =>
            `event: ${event}\nid: ${id}\ndata: ${
              typeof data === "string" ? data : JSON.stringify(data)
            }\n\n`,
        )
        .join("");
    await page.route(/\/api\/conversations\/[^/]+\/messages$/, (route) => {
      if (route.request().method() !== "POST") return route.fallback();
      return route.fulfill({
        contentType: "text/event-stream",
        body: sse([
          ["run_started", "0", { run_id: runId }],
          [
            "done",
            "1",
            {
              role: "assistant",
              content: "Berlin is sunny today.",
              message_id: messageId,
            },
          ],
        ]),
      });
    });
    await page.route(
      new RegExp(`/api/conversations/[^/]+/runs/${runId}/events/stream`),
      (route) =>
        route.fulfill({
          contentType: "text/event-stream",
          body: sse([
            [
              "suggestions",
              "2",
              {
                message_id: messageId,
                suggestions: [
                  { prompt: "What about tomorrow?" },
                  { prompt: "Show the weekly forecast" },
                  { prompt: "Should I bring an umbrella?" },
                ],
              },
            ],
          ]),
        }),
    );

    await navigateAndSettle(page, "/chat");
    await enableAccessibility(page);
    const input = page.getByRole("textbox").last();
    await input.fill("What is the weather in Berlin?");
    // The first send after a cold server start can be dropped while the app
    // is still connecting, so retry until the chips confirm it went through.
    // Flutter exposes ActionChip as a checkbox in the semantics tree.
    await expect(async () => {
      await input.press("Enter");
      await expect(
        page.getByRole("checkbox", { name: "What about tomorrow?" }),
      ).toBeVisible({ timeout: 5_000 });
    }).toPass({ timeout: 30_000 });
    await page.waitForTimeout(CSS_SETTLE_MS);
    await expect(page).toHaveScreenshot("chat-follow-up-suggestions.png", {
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

  // NOTE: /contexts is hidden on web (redirects to /chat) — managed via the
  // native app's context switcher only.

  test("integrations catalog page", async ({ page }) => {
    await mockCatalog(page, CATALOG_FIXTURE);
    await navigateAndSettle(page, "/integrations");
    await expect(page).toHaveScreenshot("integrations-catalog.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("integrations catalog page (empty state)", async ({ page }) => {
    await mockCatalog(page, []);
    await navigateAndSettle(page, "/integrations");
    await expect(page).toHaveScreenshot("integrations-empty.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("integrations catalog page (error state)", async ({ page }) => {
    await page.route(/\/api\/integrations\/catalog(\?.*)?$/, (route) =>
      route.fulfill({ status: 500, json: { error: "catalog unavailable" } }),
    );
    await navigateAndSettle(page, "/integrations");
    await expect(page).toHaveScreenshot("integrations-error.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("integrations catalog entry detail", async ({ page }) => {
    // An org admin may install Linear, so the Install button is enabled.
    await mockEverything(page, { role: "org_admin" });
    await mockCatalog(page, CATALOG_FIXTURE);
    await navigateAndSettle(page, "/integrations");
    // Flutter builds its semantics tree only after the (off-screen)
    // accessibility placeholder is activated; the tile then shows up as a
    // button whose label holds its text.
    await enableAccessibility(page);
    await page
      .getByRole("button", { name: /Linear/ })
      .first()
      .click();
    await page.waitForTimeout(CSS_SETTLE_MS * 2);
    await expect(page).toHaveScreenshot("integrations-detail.png", {
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
    await navigateAndSettle(page, "/personas/new");
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

  test("skill create form", async ({ page }) => {
    await navigateAndSettle(page, "/skills/new");
    await expect(page).toHaveScreenshot("skill-create-form.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("skill edit form", async ({ page }) => {
    const skillName = await createSkill(page);
    await navigateAndSettle(page, `/skills/${skillName}/edit`);
    await expect(page).toHaveScreenshot("skill-edit-form.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("personas list page", async ({ page }) => {
    await navigateAndSettle(page, "/personas");
    await expect(page).toHaveScreenshot("personas-list.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("persona detail screen", async ({ page }) => {
    const personaId = await createPersona(page);
    await navigateAndSettle(page, `/personas/${personaId}`);
    await expect(page).toHaveScreenshot("persona-detail.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("settings screen", async ({ page }) => {
    await mockSettingsAdmin(page);
    await navigateAndSettle(page, "/settings");
    // The section reloads once the space selection resolves, so a second
    // request reaches the mock. Wait for that as well before the screenshot.
    await page.waitForTimeout(1_500);
    // The settings list scrolls inside the Flutter canvas, so bring the tool
    // approvals (and the admin-only members group) into view. Desktop and
    // tablet take wheel events; touch emulation ignores them, so also drive a
    // synthesized touch scroll through CDP.
    const viewport = page.viewportSize() ?? { width: 1280, height: 900 };
    const scrollX = Math.round(viewport.width * 0.75);
    const scrollY = Math.round(viewport.height * 0.6);
    await page.mouse.move(scrollX, scrollY);
    await page.mouse.wheel(0, 2_000);
    await page.waitForTimeout(500);
    const cdp = await page.context().newCDPSession(page);
    for (let i = 0; i < 2; i++) {
      await cdp.send("Input.synthesizeScrollGesture", {
        x: scrollX,
        y: scrollY,
        yDistance: -2000,
        speed: 2000,
      });
      await page.waitForTimeout(700);
    }
    await expect(page).toHaveScreenshot("settings.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("account screen", async ({ page }) => {
    await navigateAndSettle(page, "/settings/account");
    // Account screen depends on two async API calls (getCurrentUser + getOrg)
    // before the form renders. In the e2e environment the test-token mode
    // returns 404 from /api/users/me (no real user record), so the screen
    // settles into its error state. Wait long enough for Dio to surface
    // the error so we don't bake in the transient loading spinner.
    await page.waitForTimeout(FLUTTER_SETTLE_MS * 2);
    await expect(page).toHaveScreenshot("account.png", {
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

  test("workflow editor - new workflow (all viewports)", async ({ page }) => {
    await navigateAndSettle(page, "/workflows/new");
    await expect(page).toHaveScreenshot("workflow-editor-new.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("workflow editor - mobile card list (linear 3-node workflow)", async ({
    page,
  }) => {
    const workflowId = await createLinearWorkflow(page);
    await navigateAndSettle(page, `/workflows/${workflowId}/edit`);
    await expect(page).toHaveScreenshot(
      "workflow-editor-mobile-card-list.png",
      {
        fullPage: true,
        maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
      },
    );
  });

  test("workflow editor - complex DAG banner (mobile)", async ({ page }) => {
    const workflowId = await createComplexWorkflow(page);
    await navigateAndSettle(page, `/workflows/${workflowId}/edit`);
    await expect(page).toHaveScreenshot("workflow-editor-complex-dag.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });

  test("core routes avoid viewport horizontal overflow", async ({ page }) => {
    // 8 routes × ~5 s each (3 s settle + navigation) — extend the default 30 s limit.
    test.setTimeout(90_000);
    const routes = [
      "/chat",
      "/agents",
      "/webhooks",
      "/workflows",
      // /contexts is redirected to /chat on web — excluded from overflow check.
      "/skills",
      "/personas",
      "/settings",
      "/settings/account",
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

test.describe("Authenticated pages (light theme)", () => {
  test.use({ colorScheme: "light" });

  test.beforeEach(async ({ page }) => {
    await loginFlutter(page);
  });

  test("integrations catalog page (light)", async ({ page }) => {
    await mockCatalog(page, CATALOG_FIXTURE);
    await navigateAndSettle(page, "/integrations");
    await expect(page).toHaveScreenshot("integrations-catalog-light.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });
});
