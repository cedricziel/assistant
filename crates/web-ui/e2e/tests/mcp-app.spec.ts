import { test, expect, Page } from "@playwright/test";

import { STAMP } from "./support/integration-mocks";

/**
 * An MCP App from a real MCP server, rendered below its tool call.
 *
 * The e2e server lists examples/mcp-apps-demo under [mcp], and its LLM is the
 * scripted stand-in in support/mock-ollama.mjs. A turn asking for the weather
 * calls the demo's get-forecast tool, so the stored tool call carries the app
 * page the server built with the official MCP Apps SDK. Its tool input and
 * result show only after the full ui/initialize handshake; its Save button
 * calls the app-only save-favorite tool back through the host.
 */

const AUTH_TOKEN = "test-token";
const MAX_DIFF_RATIO_FLUTTER = 0.15;
const FLUTTER_SETTLE_MS = 3000;
const CSS_SETTLE_MS = 300;

/** Runs one turn through the real server and returns its conversation id. */
async function runForecastTurn(baseURL: string): Promise<string> {
  const headers = {
    Authorization: `Bearer ${AUTH_TOKEN}`,
    "Content-Type": "application/json",
  };
  const created = await fetch(`${baseURL}/api/conversations`, {
    method: "POST",
    headers,
    body: JSON.stringify({ title: "Weather in Berlin" }),
  });
  expect(created.status).toBe(201);
  const { id } = await created.json();

  // A separate orchestrator process runs the turn, and this stream gets no
  // frames from it, so wait on the stored history instead.
  const turn = await fetch(`${baseURL}/api/conversations/${id}/messages`, {
    method: "POST",
    headers,
    body: JSON.stringify({ message: "What is the weather in Berlin?" }),
  });
  expect(turn.status).toBe(200);
  await expect
    .poll(
      async () => {
        const detail = await fetch(`${baseURL}/api/conversations/${id}`, {
          headers,
        }).then((r) => r.json());
        return detail.messages.flatMap(
          (m: { tool_calls?: { app?: { resource_uri: string } }[] }) =>
            (m.tool_calls ?? []).map((c) => c.app?.resource_uri),
        );
      },
      { message: "the turn stores the demo tool call with its app" },
    )
    .toEqual(["ui://demo/forecast.html"]);
  await turn.body?.cancel();
  return id;
}

/**
 * Shows only this conversation in the list and pins its timestamps, so the
 * screenshot does not depend on other tests or the clock. The conversation
 * detail itself still comes from the server.
 */
async function pinConversation(page: Page, id: string) {
  const pinStamps = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.map(pinStamps);
    if (value && typeof value === "object") {
      return Object.fromEntries(
        Object.entries(value).map(([k, v]) => [
          k,
          k.endsWith("_at") ? STAMP : pinStamps(v),
        ]),
      );
    }
    return value;
  };
  await page.route(/\/api\/conversations\/stream(\?.*)?$/, (route) =>
    route.fulfill({
      contentType: "text/event-stream",
      body: `event: snapshot\ndata: ${JSON.stringify([
        {
          id,
          title: "Weather in Berlin",
          created_at: STAMP,
          updated_at: STAMP,
        },
      ])}\n\n`,
    }),
  );
  await page.route(
    new RegExp(`/api/conversations/${id}(\\?.*)?$`),
    async (route) => {
      if (route.request().method() !== "GET") return route.fallback();
      const response = await route.fetch();
      await route.fulfill({ response, json: pinStamps(await response.json()) });
    },
  );
}

async function loginFlutter(page: Page) {
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "load" });
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 20_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

/** Flutter builds its semantics tree only once this placeholder is clicked. */
async function enableAccessibility(page: Page) {
  await page
    .getByRole("button", { name: "Enable accessibility" })
    .evaluate((el) => (el as HTMLElement).click());
}

test.describe("MCP Apps", () => {
  test.beforeEach(async ({}, testInfo) => {
    test.skip(
      !["desktop-chrome", "mobile-chrome"].includes(testInfo.project.name),
      "desktop and mobile widths only",
    );
  });

  test("app from the demo server renders and calls its server", async ({
    page,
    baseURL,
  }) => {
    const id = await runForecastTurn(baseURL!);
    await pinConversation(page, id);
    await loginFlutter(page);
    await page.goto(`/chat/${id}`, { waitUntil: "load" });

    const frame = page.locator('iframe[sandbox="allow-scripts"]');
    await expect(frame).toHaveCount(1, { timeout: 20_000 });
    const app = frame.contentFrame();
    await expect(app.getByText("Berlin", { exact: true })).toBeVisible();
    await expect(app.getByText("21°C")).toBeVisible();
    await expect(app.getByText("Sunny with a light breeze")).toBeVisible();
    const save = app.getByRole("button", { name: "Save as favorite" });
    await expect(save).toBeEnabled();

    await page.waitForTimeout(FLUTTER_SETTLE_MS + CSS_SETTLE_MS);
    // Viewport only: with the app's iframe on the page, a full-page capture
    // grows past the 1280x900 document.
    await expect(page).toHaveScreenshot("mcp-app-history.png", {
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });

    // save-favorite is not read-only: the server answers 409 until the user
    // confirms, then runs it on the demo server.
    const appCalls: number[] = [];
    page.on("response", (response) => {
      if (response.url().endsWith("/app/tools-call")) {
        appCalls.push(response.status());
      }
    });
    await save.click();
    await enableAccessibility(page);
    await expect(
      page.getByText('Allow demo to run "save-favorite"?'),
    ).toBeVisible();
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await expect(
      app.getByText("Saved Berlin to your favorites."),
    ).toBeVisible();
    expect(appCalls).toEqual([409, 200]);
  });
});
