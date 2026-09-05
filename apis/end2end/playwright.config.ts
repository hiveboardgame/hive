import { defineConfig, devices } from "playwright/test";

/**
 * Read environment variables from file.
 * https://github.com/motdotla/dotenv
 */
// require('dotenv').config();

/**
 * See https://playwright.dev/docs/test-configuration.
 */
const config = defineConfig({
  testDir: "./tests",
  /* A development HiveGame.wasm bundle is large and must hydrate first. */
  timeout: 15 * 1000,
  expect: {
    /**
     * Maximum time expect() should wait for the condition to be met.
     * For example in `await expect(locator).toHaveText();`
     */
    timeout: 5000,
  },
  /* Tests in the same file may run in parallel when more are added. */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /*
   * Each project downloads and compiles HiveGame.wasm. Keep the initial smoke
   * suite serial so the desktop and mobile projects do not compete for four
   * simultaneous large WASM downloads.
   */
  workers: 1,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: "html",
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Maximum time each action such as `click()` can take. Defaults to 0 (no limit). */
    actionTimeout: 0,
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:3000",

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: "on-first-retry",
  },

  /* The app is started by the developer before the suite is run. */

  /* Cover Chromium, Firefox, and WebKit at desktop and mobile layouts. */
  projects: [
    {
      name: "chromium-desktop",
      use: {
        ...devices["Desktop Chrome"],
      },
    },

    {
      name: "firefox-desktop",
      use: {
        ...devices["Desktop Firefox"],
      },
    },

    {
      name: "webkit-desktop",
      use: {
        ...devices["Desktop Safari"],
      },
    },

    {
      name: "chromium-mobile",
      use: {
        ...devices["Pixel 5"],
      },
    },

    // Playwright does not provide Firefox for Android; this is Firefox at the
    // same narrow viewport as Pixel 5 so responsive layout is covered.
    {
      name: "firefox-mobile",
      use: {
        ...devices["Desktop Firefox"],
        viewport: devices["Pixel 5"].viewport,
        screen: devices["Pixel 5"].screen,
      },
    },

    {
      name: "webkit-mobile",
      use: {
        ...devices["iPhone 12"],
      },
    },
  ],

  /* Folder for test artifacts such as screenshots, videos, traces, etc. */
  // outputDir: 'test-results/',

});

export default config;
