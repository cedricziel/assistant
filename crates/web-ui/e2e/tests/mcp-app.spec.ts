import { test, expect, Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

import { STAMP } from "./support/integration-mocks";

/**
 * Visual regression test for an MCP App rendered below its tool call.
 *
 * The conversation and its history are mocked, so the screenshot does not
 * depend on server state. The fixture app runs in the sandboxed iframe and
 * talks to the host bridge: the test sees its rendered tool input and result
 * only when ui/initialize, initialized, tool-input and tool-result all went
 * through.
 */

const AUTH_TOKEN = "test-token";
const MAX_DIFF_RATIO_FLUTTER = 0.15;
const FLUTTER_SETTLE_MS = 3000;
const CSS_SETTLE_MS = 300;

const CONVERSATION_ID = "0b6c2f0e-4d0a-4c55-9a43-6f3d8d1c0a01";
const APP_HTML = fs.readFileSync(
  path.join(__dirname, "fixtures", "mcp-app-weather.html"),
  "utf8",
);

const CONVERSATION = {
  id: CONVERSATION_ID,
  title: "Weather in Berlin",
  created_at: STAMP,
  updated_at: STAMP,
};

const DETAIL = {
  ...CONVERSATION,
  messages: [
    {
      id: "m1",
      role: "user",
      content: "What is the weather in Berlin?",
      turn: 0,
      created_at: STAMP,
      tts_available: false,
    },
    {
      id: "m2",
      role: "assistant",
      content: "",
      turn: 0,
      created_at: STAMP,
      tts_available: false,
      tool_calls: [
        {
          name: "mcp__weather__forecast",
          status: "ok",
          tool_call_id: "call-1",
          arguments: { city: "Berlin" },
          result: "Sunny with a light breeze",
          data: { temperature: 21 },
          app: {
            server: "weather",
            resource_uri: "ui://weather/forecast",
            html: APP_HTML,
            csp: {},
            permissions: {
              camera: false,
              microphone: false,
              geolocation: false,
              clipboard_write: false,
            },
          },
        },
      ],
    },
    {
      id: "m3",
      role: "assistant",
      content: "It is sunny and 21°C in Berlin.",
      turn: 0,
      created_at: STAMP,
      tts_available: false,
    },
  ],
};

async function mockConversation(page: Page) {
  const json = (body: unknown) => ({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify(body),
  });
  await page.route(/\/api\/conversations(\?.*)?$/, (route) =>
    route.request().method() === "GET"
      ? route.fulfill(json([CONVERSATION]))
      : route.fallback(),
  );
  await page.route(
    new RegExp(`/api/conversations/${CONVERSATION_ID}(\\?.*)?$`),
    (route) => route.fulfill(json(DETAIL)),
  );
}

async function loginFlutter(page: Page) {
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "load" });
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 20_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

test.describe("MCP Apps", () => {
  test.beforeEach(async ({}, testInfo) => {
    test.skip(
      !["desktop-chrome", "mobile-chrome"].includes(testInfo.project.name),
      "desktop and mobile widths only",
    );
  });

  test("app renders below its tool call", async ({ page }) => {
    await mockConversation(page);
    await loginFlutter(page);
    await page.goto(`/chat/${CONVERSATION_ID}`, { waitUntil: "load" });

    const frame = page.locator('iframe[sandbox="allow-scripts"]');
    await expect(frame).toHaveCount(1, { timeout: 20_000 });
    const app = frame.contentFrame();
    await expect(app.getByText("21°C")).toBeVisible();
    await expect(app.getByText("Berlin", { exact: true })).toBeVisible();
    await expect(app.getByText("Sunny with a light breeze")).toBeVisible();

    await page.waitForTimeout(FLUTTER_SETTLE_MS + CSS_SETTLE_MS);
    await expect(page).toHaveScreenshot("mcp-app-history.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });
  });
});
