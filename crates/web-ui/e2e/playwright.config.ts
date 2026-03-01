import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright configuration for visual regression tests.
 *
 * The web-ui server must be running at BASE_URL before tests start.
 * Use the helper script `./run-server.sh` or start it manually:
 *
 *   cargo run -p assistant-web-ui -- --auth-token test-token --listen 127.0.0.1:8787
 *
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

  /* Reporter */
  reporter: process.env.CI ? "github" : "html",

  use: {
    baseURL: process.env.E2E_BASE_URL || "http://127.0.0.1:8787",
    /* Consistent viewport for snapshot stability */
    viewport: { width: 1280, height: 900 },
    colorScheme: "dark",
    /* Reduce flakiness from animations */
    actionTimeout: 10_000,
  },

  projects: [
    {
      name: "desktop-chrome",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  /* Auto-start the server if not already running */
  webServer: {
    command:
      "cargo run -p assistant-web-ui -- --auth-token test-token --listen 127.0.0.1:8787 --db-path /tmp/assistant-e2e-test.db",
    url: "http://127.0.0.1:8787/login",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
