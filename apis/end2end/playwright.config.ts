import { defineConfig, devices } from "@playwright/test";

const artifacts = `test-results/runs/${process.env.PLAYWRIGHT_RUN_ID ?? "attached"}`;
const fullMatrix = process.env.PLAYWRIGHT_FULL_MATRIX === "1";
const compatibility = fullMatrix ? undefined : /@interaction|@smoke/;

export default defineConfig({
  testDir: "./tests",
  outputDir: `${artifacts}/browser`,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  failOnFlakyTests: true,
  retries: 0,
  workers: process.env.CI ? 1 : 2,
  timeout: 45_000,
  expect: { timeout: 10_000 },
  // Per-test deadlines remain bounded; deliberate repeats may take arbitrarily long.
  // CI also has its job deadline, and callers can set --global-timeout when needed.
  reporter: [["list"], ["html", { open: "never", outputFolder: `${artifacts}/report` }], ["junit", { outputFile: `${artifacts}/results.xml` }]],
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL,
    viewport: { width: 1280, height: 900 },
    // PWA caching/offline-shell behavior has its own scope; keep these interaction runs uncached.
    serviceWorkers: "block",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [
    { name: "chromium-desktop", use: { ...devices["Desktop Chrome"] }, grepInvert: /@chromium-touch/ },
    { name: "firefox-desktop", use: { ...devices["Desktop Firefox"] }, grep: compatibility, grepInvert: /@chromium-touch/ },
    { name: "webkit-desktop", use: { ...devices["Desktop Safari"] }, grep: compatibility, grepInvert: /@chromium-touch/ },
    { name: "chromium-mobile", use: { ...devices["Pixel 5"], viewport: { width: 390, height: 844 } }, grepInvert: /@mouse/ },
    // Narrow desktop Firefox checks responsive layout, not Firefox Android or touch input.
    { name: "firefox-narrow", use: { ...devices["Desktop Firefox"], viewport: { width: 390, height: 844 } }, grep: compatibility, grepInvert: /@chromium-touch/ },
    { name: "webkit-mobile", use: { ...devices["iPhone 12"], viewport: { width: 390, height: 844 } }, grep: compatibility, grepInvert: /@mouse|@chromium-touch/ },
  ],
});
