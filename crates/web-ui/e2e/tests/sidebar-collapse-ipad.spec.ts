import { test, expect, Page } from "@playwright/test";

import { enableAccessibility } from "./support/flutter";

/**
 * Sidebar-collapse persistence regression for iPad-landscape viewport (#620).
 *
 * playwright.config.ts runs this file only on the `ipad-landscape-chrome`
 * project.
 */

const AUTH_TOKEN = "test-token";
const FLUTTER_SETTLE_MS = 3000;

// Flutter builds its semantics tree, which holds the tooltip labels the
// tests query, only after its off-screen accessibility placeholder is clicked.
async function loginFlutter(page: Page) {
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "load" });
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 20_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
  await enableAccessibility(page);
}

async function reloadFlutter(page: Page) {
  await page.reload({ waitUntil: "load" });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
  await enableAccessibility(page);
}

test.describe("iPad-landscape sidebar collapse (#620)", () => {
  test("top-leading toggle is visible and toggles the sidebar", async ({
    page,
  }) => {
    await loginFlutter(page);

    // Top-leading toggle is rendered as a Flutter IconButton; Flutter exposes
    // tooltips as accessible labels. `getByRole('button', { name: ... })`
    // matches the tooltip text rendered by `Tooltip` widgets.
    const hideButton = page.getByRole("button", { name: "Hide navigation" });
    await expect(hideButton).toBeVisible();

    await hideButton.click();
    await page.waitForTimeout(400); // animation

    const showButton = page.getByRole("button", { name: "Show navigation" });
    await expect(showButton).toBeVisible();
  });

  test("collapse state persists across reload", async ({ page }) => {
    await loginFlutter(page);

    // Start collapsed.
    await page.getByRole("button", { name: "Hide navigation" }).click();
    await page.waitForTimeout(400);

    // Reload — Flutter rebuilds, sidebar provider rehydrates from
    // SharedPreferences (localStorage on web).
    await reloadFlutter(page);

    // After reload the collapsed state should be preserved.
    await expect(
      page.getByRole("button", { name: "Show navigation" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Hide navigation" }),
    ).toHaveCount(0);
  });

  test("re-expanding clears the persisted collapse", async ({ page }) => {
    await loginFlutter(page);

    await page.getByRole("button", { name: "Hide navigation" }).click();
    await page.waitForTimeout(400);
    await page.getByRole("button", { name: "Show navigation" }).click();
    await page.waitForTimeout(400);

    await reloadFlutter(page);

    await expect(
      page.getByRole("button", { name: "Hide navigation" }),
    ).toBeVisible();
  });
});
