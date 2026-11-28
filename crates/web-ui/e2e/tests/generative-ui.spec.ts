import { test, expect, Page } from "@playwright/test";

import { enableAccessibility } from "./support/flutter";
import { STAMP } from "./support/integration-mocks";

/**
 * A generative-UI surface from a real turn, rendered in the web chat.
 *
 * The e2e server's LLM is the scripted stand-in in support/mock-ollama.mjs. A
 * turn asking for a surface calls the real `render-surface` tool, so the
 * server validates, persists and streams it like any other turn. The surface
 * holds a Button whose `openUrl` targets example.org, which is not on the
 * org's link allowlist, so tapping it asks first. Sending a follow-up message
 * then closes the surface in place.
 */

const AUTH_TOKEN = "test-token";
const MAX_DIFF_RATIO_FLUTTER = 0.15;
const FLUTTER_SETTLE_MS = 3000;
const CSS_SETTLE_MS = 300;
const LINK = "https://example.org/berlin/events";

/** Runs one turn through the real server and returns its conversation id. */
async function runSurfaceTurn(baseURL: string): Promise<string> {
  const headers = {
    Authorization: `Bearer ${AUTH_TOKEN}`,
    "Content-Type": "application/json",
  };
  const created = await fetch(`${baseURL}/api/conversations`, {
    method: "POST",
    headers,
    body: JSON.stringify({ title: "Weekend in Berlin" }),
  });
  expect(created.status).toBe(201);
  const { id } = await created.json();

  // A separate orchestrator process runs the turn, and this stream gets no
  // frames from it, so wait on the stored history instead.
  const turn = await fetch(`${baseURL}/api/conversations/${id}/messages`, {
    method: "POST",
    headers,
    body: JSON.stringify({ message: "Show me the events surface" }),
  });
  expect(turn.status).toBe(200);
  await expect
    .poll(
      async () => {
        const detail = await fetch(`${baseURL}/api/conversations/${id}`, {
          headers,
        }).then((r) => r.json());
        const surfaceStored = (detail.surfaces?.length ?? 0) === 1;
        const turnFinished = (detail.messages ?? []).some(
          (m: { content?: string }) =>
            m.content === "Here are the events for this weekend.",
        );
        return surfaceStored && turnFinished;
      },
      { message: "the turn stores the surface and its reply", timeout: 30_000 },
    )
    .toBe(true);
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
          title: "Weekend in Berlin",
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

test.describe("Generative UI surfaces", () => {
  test.beforeEach(async ({}, testInfo) => {
    test.skip(
      !["desktop-chrome", "mobile-chrome"].includes(testInfo.project.name),
      "desktop and mobile widths only",
    );
  });

  test("surface asks before an external link and closes after a new message", async ({
    page,
    baseURL,
  }) => {
    // The scripted turn runs in a separate orchestrator process.
    test.slow();
    const id = await runSurfaceTurn(baseURL!);
    await pinConversation(page, id);
    await loginFlutter(page);
    await page.goto(`/chat/${id}`, { waitUntil: "load" });
    await enableAccessibility(page);

    await expect(
      page.getByRole("button", { name: "Open details" }),
    ).toBeVisible({ timeout: 20_000 });
    await page.waitForTimeout(FLUTTER_SETTLE_MS + CSS_SETTLE_MS);
    await expect(page).toHaveScreenshot("surface-active.png", {
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });

    // example.org is not in the org's empty `[ui] link_hosts`, so the tap
    // shows the destination and waits for a second gesture.
    await page.getByRole("button", { name: "Open details" }).click();
    await expect(page.getByText("Open link?")).toBeVisible();
    await expect(page.getByText(LINK, { exact: false })).toBeVisible();
    await page.waitForTimeout(CSS_SETTLE_MS);
    await expect(page).toHaveScreenshot("surface-link-confirmation.png", {
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(page.getByText("Open link?")).toBeHidden();

    // A later user message closes the surface. Wait for the turn's SSE body to
    // finish so the screenshot captures the settled timeline; the committed
    // baseline covers the rendered "Closed" marker. The first send after a
    // cold start can be dropped while the app connects, so retry until one
    // reaches the server.
    const input = page.getByRole("textbox").last();
    let response: Awaited<ReturnType<typeof page.waitForResponse>> | undefined;
    await expect(async () => {
      await input.fill("Thanks!");
      const sent = page.waitForResponse(
        (candidate) =>
          candidate.url().endsWith(`/api/conversations/${id}/messages`) &&
          candidate.request().method() === "POST",
        { timeout: 3_000 },
      );
      await input.press("Enter");
      response = await sent;
      expect(response.status()).toBe(200);
    }).toPass({ timeout: 60_000 });
    await response!.finished();
    await page.waitForTimeout(FLUTTER_SETTLE_MS);
    await expect(page).toHaveScreenshot("surface-closed.png", {
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });
});
