import { test, expect } from "../support/fixtures";
import { expectHydrated } from "../support/browser/hydration";
import { activate } from "../support/gestures";

test("a guest can navigate and is asked to sign in before playing", { tag: "@smoke" }, async ({ page, isMobileLayout, hasTouch }) => {
  await page.goto("/");
  await expectHydrated(page);
  await activate(page.getByRole("button", { name: isMobileLayout ? "Open navigation menu" : "Learn", exact: true }), hasTouch);
  await activate(page.getByRole("link", { name: "Analysis", exact: true }), hasTouch);
  await expect(page).toHaveURL(/\/analysis$/);
  await expect(page.getByTestId("hive-board")).toBeVisible();
  if (isMobileLayout) {
    await activate(page.getByRole("button", { name: "Open navigation menu", exact: true }), hasTouch);
  }
  await activate(page.getByRole("link", { name: "Home", exact: true }), hasTouch);
  await activate(page.getByRole("button", { name: "1+2", exact: true }), hasTouch);
  await expect(page).toHaveURL(/\/login(?:\?|$)/);
  await expect(page.getByRole("heading", { name: "Sign in", exact: true })).toBeVisible();
});
