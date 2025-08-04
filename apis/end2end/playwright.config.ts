import type { PlaywrightTestConfig } from "@playwright/test";
import { devices } from "@playwright/test";

/// <reference types="node" />
const config: PlaywrightTestConfig = {
  testDir: "./tests",
  timeout: 60 * 1000,
  expect: {
    timeout: 10000,
  },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : 2,
  reporter: "html",
  use: {
    actionTimeout: 15000,
    baseURL: "http://0.0.0.0:3000",

    trace: "on-first-retry",

    screenshot: "only-on-failure",

    ignoreHTTPSErrors: true,

    navigationTimeout: 30000,
  },

  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
      },
    },

    {
      name: "firefox",
      use: {
        ...devices["Desktop Firefox"],
      },
    },

    {
      name: 'Mobile Chrome',
      use: {
        ...devices['Pixel 5'],
      },
    },

  ],

  outputDir: 'test-results/',

  webServer: {
    command: 'cd .. && LEPTOS_TAILWIND_VERSION=v3.4.1 cargo leptos watch',
    port: 3000,
    timeout: 600 * 1000,
    reuseExistingServer: !process.env.CI,
  },
};

export default config;
