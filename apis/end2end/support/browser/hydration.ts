import { expect, type Page } from "@playwright/test";

export async function expectHydrated(page: Page) {
  // The app reveals its main content during hydration. A document load can
  // finish before WASM compilation, so keep that wait separate from interactions.
  await expect(
    page.getByRole("main"),
    "The hydrated page shows its main content",
  ).toBeVisible({ timeout: 45_000 });
}

export async function reloadHydrated(page: Page) {
  await page.reload();
  await expectHydrated(page);
}
