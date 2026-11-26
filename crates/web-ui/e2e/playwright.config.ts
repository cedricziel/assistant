import { defineConfig, devices } from "@playwright/test";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

// The server keeps all state under $HOME/.assistant, so a throwaway HOME
// isolates each run. Playwright loads this file in the runner and again in
// every worker; workers inherit the runner's environment, so only the runner
// creates the directory, and it removes it when it exits.
if (!process.env.ASSISTANT_E2E_HOME) {
  const created = fs.mkdtempSync(path.join(os.tmpdir(), "assistant-e2e-home-"));
  process.env.ASSISTANT_E2E_HOME = created;
  process.on("exit", () => fs.rmSync(created, { recursive: true, force: true }));
}
const homeDir = process.env.ASSISTANT_E2E_HOME;
const assistant =
  process.env.ASSISTANT_BIN || "cargo run -q -p assistant-cli --";
const MOCK_LLM_PORT = 18788;

// The iPad-landscape project exists only for the sidebar-collapse regression
// (#620); it has no screenshot baselines.
const IPAD_ONLY_SPEC = /sidebar-collapse-ipad\.spec\.ts/;

/**
 * Playwright configuration for visual regression tests.
 *
 * Three viewport sizes match the app's responsive breakpoints:
 *   - Desktop (1280px) — full icon rail + top bar
 *   - Tablet  (768px)  — hamburger menu, no icon rail  (breakpoint: 900px)
 *   - Mobile  (375px)  — bottom tab bar, no top bar    (breakpoint: 640px)
 *
 * A fourth project, iPad landscape (1180px), runs only the sidebar-collapse spec.
 *
 * The assistant webui server must be running at BASE_URL before tests start.
 * Set E2E_BASE_URL to override the default address.
 */
export default defineConfig({
  testDir: "./tests",
  outputDir: "./test-results",
  snapshotDir: "./screenshots",
  snapshotPathTemplate: "{snapshotDir}/{testFilePath}/{arg}{-projectName}{ext}",

  /* Fail fast in CI, retry locally */
  retries: process.env.CI ? 0 : 1,
  workers: process.env.CI ? 1 : undefined,
  timeout: 30_000,

  /* Reporter: always generate HTML report for visual diff review.
   * In CI, also emit GitHub annotations for inline failure markers. */
  reporter: process.env.CI
    ? [
        ["github"],
        ["html", { open: "never", outputFolder: "playwright-report" }],
      ]
    : [["html", { open: "on-failure", outputFolder: "playwright-report" }]],

  use: {
    baseURL: process.env.E2E_BASE_URL || "http://127.0.0.1:8787",
    colorScheme: "dark",
    /* Reduce flakiness from animations */
    actionTimeout: 10_000,
    /* Block service workers so Flutter web always loads fresh assets during tests,
     * preventing stale caches from serving outdated code after rebuilds. */
    serviceWorkers: "block",
  },

  projects: [
    {
      name: "desktop-chrome",
      testIgnore: IPAD_ONLY_SPEC,
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1280, height: 900 },
      },
    },
    {
      name: "tablet-chrome",
      testIgnore: IPAD_ONLY_SPEC,
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 768, height: 1024 },
      },
    },
    {
      name: "ipad-landscape-chrome",
      testMatch: IPAD_ONLY_SPEC,
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1180, height: 820 },
      },
    },
    {
      name: "mobile-chrome",
      testIgnore: IPAD_ONLY_SPEC,
      use: {
        ...devices["Pixel 7"],
      },
    },
  ],

  /* Auto-start the servers if not already running: a scripted Ollama
   * stand-in, then the assistant web UI and orchestrator with the
   * examples/mcp-apps-demo MCP server under [mcp] and the stand-in as their
   * LLM (see tests/support/e2e-server.mjs). */
  webServer: [
    {
      command: `node tests/support/mock-ollama.mjs ${MOCK_LLM_PORT}`,
      url: `http://127.0.0.1:${MOCK_LLM_PORT}/api/tags`,
      reuseExistingServer: !process.env.CI,
      timeout: 10_000,
    },
    {
      // The server reads its HOME from ASSISTANT_E2E_HOME, inherited
      // from this process.
      command: `node tests/support/e2e-server.mjs ${MOCK_LLM_PORT} ${assistant}`,
      url: "http://127.0.0.1:8787/login",
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
      stdout: "pipe",
      stderr: "pipe",
    },
  ],
});
