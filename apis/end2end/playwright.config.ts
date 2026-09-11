import { defineConfig, devices } from "@playwright/test";

// Start the Hive server before running the suite.
export default defineConfig({
  testDir: "./tests",
  outputDir: "./test-results/e2e-tests",
  /* A development HiveGame.wasm bundle is large and must hydrate first. */
  timeout: 15_000,
  expect: { timeout: 5000 },
  /* Authenticated tests reserve one or two exclusive accounts from the shared pools. */
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 4,
  reporter: process.env.CI
    ? [
        ["list", { printSteps: true, printFailuresInline: true }],
        ["html", { open: "never" }],
        ["github"],
        ["./reporters/github-summary.ts"],
      ]
    : [["html"], ["./reporters/github-summary.ts"]],
  use: {
    actionTimeout: 0,
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:3000",
    // Opt in only for a local HTTPS server with a self-signed certificate.
    ignoreHTTPSErrors: process.env.PLAYWRIGHT_IGNORE_HTTPS_ERRORS === "1",
    // Service workers can bypass the WebKit session-cookie response interceptor.
    serviceWorkers: "block",
    trace: "on-first-retry",
  },

  projects: [
    { name: "chromium-desktop", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox-desktop", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit-desktop", use: { ...devices["Desktop Safari"] } },
    { name: "chromium-mobile", use: { ...devices["Pixel 5"] } },
    // Playwright does not provide Firefox for Android; this is Firefox at the
    // same narrow viewport as Pixel 5 so responsive layout is covered.
    {
      name: "firefox-mobile",
      use: {
        ...devices["Desktop Firefox"],
        viewport: devices["Pixel 5"].viewport,
      },
    },
    { name: "webkit-mobile", use: { ...devices["iPhone 12"] } },
  ],
});
