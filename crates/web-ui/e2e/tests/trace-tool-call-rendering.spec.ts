import { test, expect, Page } from "@playwright/test";

/**
 * Trace detail — tool-call cards (#265).
 *
 * Stub the `/api/traces/{id}` response so we don't need a live conversation
 * to verify the new ToolCallSpanCard rendering. The Flutter view layer
 * branches on `name.startsWith('execute_tool ')`, so a hand-crafted span
 * payload is sufficient to exercise the card.
 */

const AUTH_TOKEN = "test-token";
const FLUTTER_SETTLE_MS = 3000;

async function loginFlutter(page: Page) {
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "load" });
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 20_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

const stubTrace = {
  trace_id: "trace-stub",
  persona_id: "persona-stub",
  start_time: "2026-01-01T00:00:00Z",
  duration_ms: 300,
  spans: [
    {
      span_id: "s1",
      name: "execute_tool file-read",
      start_time: "2026-01-01T00:00:00Z",
      duration_ms: 12,
      attributes: {
        tool_name: "file-read",
        tool_status: "ok",
        tool_params: '{"path":"/tmp/foo.txt"}',
        tool_observation: "ok, 42 bytes",
        iteration: 1,
        turn: 1,
        interface: "Cli",
      },
    },
    {
      span_id: "s2",
      name: "execute_tool bash",
      start_time: "2026-01-01T00:00:01Z",
      duration_ms: 8,
      attributes: {
        tool_name: "bash",
        tool_status: "error",
        tool_error: "permission denied",
        iteration: 1,
        turn: 2,
        interface: "Cli",
      },
    },
    {
      span_id: "s3",
      name: "chat anthropic/claude",
      start_time: "2026-01-01T00:00:02Z",
      duration_ms: 200,
      attributes: { "gen_ai.provider.name": "anthropic" },
    },
  ],
};

test.describe("Trace tool-call rendering (#265)", () => {
  test("dedicated cards for tool spans with status pills", async ({ page }) => {
    await page.route("**/api/traces/trace-stub", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(stubTrace),
      }),
    );

    await loginFlutter(page);
    await page.goto("/traces/trace-stub", { waitUntil: "load" });
    await page.waitForTimeout(FLUTTER_SETTLE_MS);

    // Tool names appear as card titles.
    await expect(page.getByText("file-read", { exact: true })).toBeVisible();
    await expect(page.getByText("bash", { exact: true })).toBeVisible();

    // Status pills.
    await expect(page.getByText("ok", { exact: true })).toBeVisible();
    await expect(page.getByText("error", { exact: true })).toBeVisible();

    // Generic chat span still rendered.
    await expect(
      page.getByText("chat anthropic/claude", { exact: true }),
    ).toBeVisible();
  });
});
