import type { Page } from "@playwright/test";

export async function confirmControl(page: Page, title: string) {
  const control = page.getByTitle(title);
  await control.click();
  await control.click();
}

export async function showControlsIfMobile(page: Page, isMobile: boolean) {
  if (isMobile) {
    await page.getByTitle("Show controls").click();
  }
}
