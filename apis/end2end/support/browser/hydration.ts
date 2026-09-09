import { expect, type Page } from "@playwright/test";

export async function expectHydrated(page: Page) {
  // The SSR shell initially marks main as hidden and reveals it during
  // hydration. The development WASM bundle is large, so allow the complete
  // download and compilation before asserting the app.
  const main = page.locator("main");
  await expect(
    main,
    "The hydrated page removes the hidden state from its main content",
  ).not.toHaveClass(/(?:^|\s)hidden(?:\s|$)/, { timeout: 45 * 1000 });
  await expect(main, "The hydrated page shows its main content").toBeVisible();
}
