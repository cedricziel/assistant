import { defineConfig, devices } from "@playwright/test";
import * as os from "os";
import * as path from "path";

const dbPath = path.join(os.tmpdir(), "assistant-e2e-test.db");

/**
 * Playwright configuration for visual regression tests.
 *
 * Three viewport sizes match the app's responsive breakpoints:
 *   - Desktop (1280px) — full icon rail + top bar
 *   - Tablet  (768px)  — hamburger menu, no icon rail  (breakpoint: 900px)
 *   - Mobile  (375px)  — bottom tab bar, no top bar    (breakpoint: 640px)
 *
 * The web-ui server must be running at BASE_URL before tests start.
 * Set E2E_BASE_URL to override the default address.
 */
export default defineConfig({
  testDir: "./tests",
  outputDir: "./test-results",
  snapshotDir: "./screenshots",
  snapshotPathTemplate:
    "{snapshotDir}/{testFilePath}/{arg}{-projectName}{ext}",

  /* Fail fast in CI, retry locally */
  retries: process.env.CI ? 0 : 1,
  timeout: 30_000,

  /* Reporter: always generate HTML report for visual diff review.
   * In CI, also emit GitHub annotations for inline failure markers. */
  reporter: process.env.CI
    ? [["github"], ["html", { open: "never", outputFolder: "playwright-report" }]]
    : [["html", { open: "on-failure", outputFolder: "playwright-report" }]],

  use: {
    baseURL: process.env.E2E_BASE_URL || "http://127.0.0.1:8787",
    colorScheme: "dark",
    /* Reduce flakiness from animations */
    actionTimeout: 10_000,
  },

  projects: [
    {
      name: "desktop-chrome",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1280, height: 900 },
      },
    },
    {
      name: "tablet-chrome",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 768, height: 1024 },
      },
    },
    {
      name: "mobile-chrome",
      use: {
        ...devices["Pixel 7"],
      },
    },
  ],

  /* Auto-start the server if not already running */
  webServer: {
    command:
      `cargo run -p assistant-web-ui -- --auth-token test-token --listen 127.0.0.1:8787 --db-path ${dbPath}`,
    url: "http://127.0.0.1:8787/login",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
