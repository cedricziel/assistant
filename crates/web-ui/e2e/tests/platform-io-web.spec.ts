import { test, expect, Page } from "@playwright/test";

/**
 * Regression test for dart:io Platform usage on web.
 *
 * Flutter's `Platform` from `dart:io` throws `UnsupportedError` when accessed
 * in a browser environment. Several files (e.g. update_banner.dart) call
 * `Platform.isIOS` directly in `build()` without a `kIsWeb` guard, causing:
 *
 *   Unsupported operation: Platform._operatingSystem
 *
 * This test navigates through the app and asserts that no such error appears
 * in the browser console. It should FAIL (red) until the offending code is
 * fixed to use `kIsWeb` / `defaultTargetPlatform` instead of `dart:io`.
 */

const AUTH_TOKEN = "test-token";
const FLUTTER_SETTLE_MS = 3000;

// Pattern that matches the fatal dart:io error on web.
const PLATFORM_ERROR_PATTERN = /Platform\._operatingSystem/;

async function loginFlutter(page: Page) {
  await page.goto(`/setup?_token=${AUTH_TOKEN}`, { waitUntil: "networkidle" });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
  await page.waitForURL((url) => !url.pathname.includes("/setup"), {
    timeout: 15_000,
  });
  await page.waitForTimeout(FLUTTER_SETTLE_MS);
}

test.describe("dart:io Platform must not be used on web", () => {
  test("no Platform._operatingSystem errors after login and navigation", async ({
    page,
  }) => {
    const consoleErrors: string[] = [];

    // Collect all console messages that mention the Platform error.
    page.on("console", (msg) => {
      const text = msg.text();
      if (PLATFORM_ERROR_PATTERN.test(text)) {
        consoleErrors.push(text);
      }
    });

    // Also catch unhandled page errors (thrown as exceptions).
    page.on("pageerror", (error) => {
      if (PLATFORM_ERROR_PATTERN.test(error.message)) {
        consoleErrors.push(error.message);
      }
    });

    // 1. Login — this loads the full Flutter app and triggers provider init.
    await loginFlutter(page);

    // 2. Navigate to /chat — UpdateBannerWrapper wraps every page and calls
    //    Platform.isIOS in its build() method, which crashes on web.
    await page.goto("/chat", { waitUntil: "networkidle" });
    await page.waitForTimeout(FLUTTER_SETTLE_MS);

    expect(
      consoleErrors,
      `Expected no Platform._operatingSystem errors in the browser console, ` +
        `but found ${consoleErrors.length}:\n${consoleErrors.join("\n")}`,
    ).toHaveLength(0);
  });
});
