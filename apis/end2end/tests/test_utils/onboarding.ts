import type { Page } from "playwright/test";

export async function dismissOnboardingBanners(page: Page) {
  // Onboarding prompts can appear after hydration and cover mobile reserves.
  for (const title of ["Install HiveGame as an app", "Get notified when it's your turn"]) {
    const banner = page.locator(".ui-panel").filter({
      has: page.getByText(title, { exact: true }),
    });
    await page.addLocatorHandler(banner, async banner => {
      await banner.getByRole("button", { name: "Dismiss", exact: true }).click();
    });
  }
}
