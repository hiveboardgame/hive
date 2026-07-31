import type { PlaywrightTestConfig } from "@playwright/test";
import { devices } from "@playwright/test";

/**
 * The suite runs against an already-running server with a seeded database:
 *
 *   cargo run --bin script cleanup
 *   cargo run --bin script tournaments
 *   cargo leptos serve
 *   cargo leptos end-to-end
 *
 * `webServer` is deliberately not configured. `cargo leptos serve` takes minutes
 * from cold and needs the workspace toolchain, so starting it per run would make
 * the suite unusable locally, and the seeding has to happen in between anyway.
 */
const config: PlaywrightTestConfig = {
  testDir: "./tests",
  /*
   * Generous because hydration dominates: a debug wasm bundle is ~240MB and each
   * test gets a cold browser context, so nothing is cached between them —
   * roughly 17s per page. Against a release build this is far quicker.
   */
  timeout: 120 * 1000,
  expect: {
    timeout: 10_000,
  },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
  use: {
    actionTimeout: 0,
    baseURL: process.env.HIVE_BASE_URL ?? "http://localhost:3000",
    trace: "on-first-retry",
  },

  projects: [
    /*
     * Phone first: it is the width everything here was least verified at, and
     * horizontal overflow only appears when the viewport is narrower than the
     * content.
     */
    {
      name: "Mobile Chrome",
      use: { ...devices["Pixel 5"] },
    },
    {
      name: "Mobile Safari",
      use: { ...devices["iPhone 12"] },
    },
    /*
     * Tablet width is where the `sm:` and `md:` breakpoints start applying, so a
     * layout can be correct on a phone and on a desktop and still break here.
     */
    {
      name: "Tablet",
      use: { ...devices["iPad Mini"] },
    },
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
    /*
     * Every page again in dark, because the palette is applied per-element and a
     * token missing its dark variant is invisible until somebody switches. One
     * viewport only: a missing colour does not depend on width, and pairing both
     * themes with all four widths would double the suite for no new coverage.
     */
    {
      name: "Mobile Chrome dark",
      use: { ...devices["Pixel 5"], colorScheme: "dark" },
    },
  ],
};

export default config;
