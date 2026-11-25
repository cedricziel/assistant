import { test, expect, Page } from "@playwright/test";

import {
  integration,
  mockEverything,
  ME,
  ORG_INTEGRATIONS,
  PERSONAL_INTEGRATIONS,
  SPACE_INTEGRATIONS,
} from "./support/integration-mocks";

/**
 * Visual regression tests for installing integrations and managing them per
 * layer: the install sheet, org settings (with the policy tab), space
 * integrations and Account → Connections.
 *
 * Every API call the screens make is mocked, so the screenshots do not depend
 * on server state. Desktop and mobile widths only; each scenario runs in the
 * dark and the light theme.
 */

const AUTH_TOKEN = "test-token";
const MAX_DIFF_RATIO_FLUTTER = 0.15;
const FLUTTER_SETTLE_MS = 3000;
const CSS_SETTLE_MS = 300;

// -- Helpers ----------------------------------------------------------------

async function loginFlutter(page: Page) {
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "load" });
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 20_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

async function navigateAndSettle(page: Page, path: string) {
  await page.goto(path, { waitUntil: "load" });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

/** Flutter builds its semantics tree only once this placeholder is clicked. */
async function enableAccessibility(page: Page) {
  await page
    .getByRole("button", { name: "Enable accessibility" })
    .evaluate((el) => (el as HTMLElement).click());
}

async function shot(page: Page, name: string) {
  await page.waitForTimeout(CSS_SETTLE_MS * 2);
  await expect(page).toHaveScreenshot(`${name}.png`, {
    fullPage: true,
    maxDiffPixelRatio: MAX_DIFF_RATIO_FLUTTER,
  });
}

// -- Tests ------------------------------------------------------------------

for (const scheme of ["dark", "light"] as const) {
  test.describe(`Integration install and layers (${scheme})`, () => {
    test.use({ colorScheme: scheme });

    test.beforeEach(async ({}, testInfo) => {
      test.skip(
        !["desktop-chrome", "mobile-chrome"].includes(testInfo.project.name),
        "desktop and mobile widths only",
      );
    });

    // -- Install sheet -------------------------------------------------------

    test("install sheet", async ({ page }) => {
      await mockEverything(page, { role: "org_admin" });
      await loginFlutter(page);
      await navigateAndSettle(page, "/integrations");
      await enableAccessibility(page);
      await page
        .getByRole("button", { name: /Linear/ })
        .first()
        .click();
      await page.waitForTimeout(CSS_SETTLE_MS * 2);
      await page.getByRole("button", { name: "Install", exact: true }).click();
      await shot(page, `integrations-install-sheet-${scheme}`);
    });

    test("install sheet (installed)", async ({ page }) => {
      await mockEverything(page, { role: "org_admin" });
      await loginFlutter(page);
      await navigateAndSettle(page, "/integrations");
      await enableAccessibility(page);
      await page
        .getByRole("button", { name: /Linear/ })
        .first()
        .click();
      await page.waitForTimeout(CSS_SETTLE_MS * 2);
      await page.getByRole("button", { name: "Install", exact: true }).click();
      await page.waitForTimeout(CSS_SETTLE_MS * 2);
      await page.getByRole("button", { name: "Install", exact: true }).click();
      await shot(page, `integrations-install-installed-${scheme}`);
    });

    test("custom integration sheet", async ({ page }) => {
      await mockEverything(page, { role: "org_admin" });
      await loginFlutter(page);
      await navigateAndSettle(page, "/integrations");
      await enableAccessibility(page);
      await page
        .getByRole("button", { name: "Add custom integration" })
        .click();
      await shot(page, `integrations-install-custom-${scheme}`);
    });

    // -- Org settings --------------------------------------------------------

    test("org integrations (populated)", async ({ page }) => {
      await mockEverything(page, {
        role: "org_admin",
        integrations: ORG_INTEGRATIONS,
      });
      await loginFlutter(page);
      await navigateAndSettle(page, "/admin/integrations");
      await shot(page, `integrations-org-populated-${scheme}`);
    });

    test("org integrations (empty)", async ({ page }) => {
      await mockEverything(page, { role: "org_admin", integrations: [] });
      await loginFlutter(page);
      await navigateAndSettle(page, "/admin/integrations");
      await shot(page, `integrations-org-empty-${scheme}`);
    });

    test("org integrations (policy tab)", async ({ page }) => {
      await mockEverything(page, {
        role: "org_admin",
        policy: {
          catalog_mode: "allowlist",
          allowed_catalog_ids: ["curated/linear-app-mcp", "curated/github-mcp"],
          allow_custom_urls: true,
        },
      });
      await loginFlutter(page);
      await navigateAndSettle(page, "/admin/integrations");
      await enableAccessibility(page);
      await page.getByRole("tab", { name: "Policy" }).click();
      await shot(page, `integrations-org-policy-${scheme}`);
    });

    // -- Space settings ------------------------------------------------------

    test("space integrations (space admin)", async ({ page }) => {
      await mockEverything(page, {
        role: "space_admin",
        integrations: [
          ...SPACE_INTEGRATIONS,
          ...ORG_INTEGRATIONS.slice(0, 2),
          integration("int_zendesk", "Zendesk"),
        ],
      });
      await loginFlutter(page);
      await navigateAndSettle(page, "/admin/spaces/eng/integrations");
      await shot(page, `integrations-space-populated-${scheme}`);
    });

    test("space integrations (member, read-only)", async ({ page }) => {
      await mockEverything(page, {
        role: "member",
        integrations: [...SPACE_INTEGRATIONS, ...ORG_INTEGRATIONS.slice(0, 2)],
      });
      await loginFlutter(page);
      await navigateAndSettle(page, "/admin/spaces/eng/integrations");
      await shot(page, `integrations-space-readonly-${scheme}`);
    });

    test("space integrations (empty)", async ({ page }) => {
      await mockEverything(page, { role: "space_admin", integrations: [] });
      await loginFlutter(page);
      await navigateAndSettle(page, "/admin/spaces/eng/integrations");
      await shot(page, `integrations-space-empty-${scheme}`);
    });

    // -- Account → Connections ----------------------------------------------

    test("connections (populated)", async ({ page }) => {
      await mockEverything(page, {
        role: "member",
        integrations: [
          ...PERSONAL_INTEGRATIONS,
          ORG_INTEGRATIONS[0],
          ...SPACE_INTEGRATIONS.slice(1),
          ORG_INTEGRATIONS[1],
        ],
      });
      await loginFlutter(page);
      await navigateAndSettle(page, "/settings/account/connections");
      await shot(page, `integrations-connections-populated-${scheme}`);
    });

    test("connections (empty)", async ({ page }) => {
      await mockEverything(page, { role: "member", integrations: [] });
      await loginFlutter(page);
      await navigateAndSettle(page, "/settings/account/connections");
      await shot(page, `integrations-connections-empty-${scheme}`);
    });
  });
}
