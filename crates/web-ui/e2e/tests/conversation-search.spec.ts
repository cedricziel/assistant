import { test, expect, Page } from "@playwright/test";

import { enableAccessibility } from "./support/flutter";

/**
 * Searching conversations from the sidebar.
 *
 * Two conversations are created through the real API. Typing "lisb" into the
 * sidebar search field replaces the list with the matching conversation, its
 * title highlighted, and clearing the field brings the full list back.
 */

const AUTH_TOKEN = "test-token";
const MAX_DIFF_RATIO_FLUTTER = 0.15;
const FLUTTER_SETTLE_MS = 3000;
const LISBON_TITLE = "Lisbon trip planning";
const OTHER_TITLE = "Quarterly budget review";

async function createConversation(baseURL: string, title: string) {
  const created = await fetch(`${baseURL}/api/conversations`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${AUTH_TOKEN}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ title }),
  });
  expect(created.status).toBe(201);
}

async function loginFlutter(page: Page) {
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "load" });
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 20_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
  await enableAccessibility(page);
}

test.describe("Conversation search", () => {
  test.beforeEach(async ({}, testInfo) => {
    test.skip(
      testInfo.project.name !== "desktop-chrome",
      "the sidebar is permanently visible at desktop width only",
    );
  });

  test("typing filters the sidebar to matching conversations", async ({
    page,
    baseURL,
  }) => {
    await createConversation(baseURL!, LISBON_TITLE);
    await createConversation(baseURL!, OTHER_TITLE);
    await loginFlutter(page);

    await expect(page.getByText(OTHER_TITLE)).toBeVisible();

    await page
      .getByRole("textbox", { name: "Search conversations" })
      .fill("lisb");

    await expect(page.getByText(LISBON_TITLE)).toBeVisible();
    await expect(page.getByText(OTHER_TITLE)).toHaveCount(0);
    await expect(page).toHaveScreenshot("conversation-search-results.png", {
      fullPage: true,
      maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
    });

    await page.getByRole("button", { name: "Clear search" }).click();
    await expect(page.getByText(OTHER_TITLE)).toBeVisible();
  });
});
