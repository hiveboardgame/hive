import { expect, test } from "@playwright/test";
import { expectHydrated } from "../support/browser/hydration";

test.describe("Anonymous navigation", () => {
  // Include the 45-second hydration wait and the remaining navigation checks.
  test.describe.configure({ timeout: 60_000 });

  test("Home rendering", async ({ page }) => {
    await test.step("Load and hydrate the home page", async () => {
      const response = await page.goto("/");

      if (response === null) {
        throw new Error("The home page did not return an HTTP response");
      }

      expect(
        response.ok(),
        `The home page responds successfully (received HTTP ${response.status()})`,
      ).toBeTruthy();
      await expect(page, "The home page has the HiveGame.com browser title").toHaveTitle(
        "HiveGame.com",
      );
      await expectHydrated(page);
      await expect(
        page.getByRole("heading", { name: "Create a game" }),
        "Visitors can see the Create a game heading",
      ).toBeVisible();
      await expect(
        page.getByRole("group", { name: "Quick play" }),
        "Visitors can see the Quick play game controls",
      ).toBeVisible();
      await expect(
        page.getByRole("link", { name: "Login", exact: true }),
        "Visitors can see the Login link",
      ).toBeVisible();
    }, { box: true });
  });

  test("Responsive navigation", async ({ page }, testInfo) => {
    await page.goto("/");
    await expectHydrated(page);
    await test.step("Use the navigation controls", async () => {
      if (testInfo.project.name.endsWith("-mobile")) {
        await page.getByRole("button", { name: "Open navigation menu" }).click();
        const navigationMenu = page.getByRole("menu");
        await expect(navigationMenu, "Opening mobile navigation shows the navigation menu").toBeVisible();
        await expect(
          navigationMenu.getByRole("link", { name: "Top Players", exact: true }),
          "The open mobile navigation includes the Top Players link",
        ).toBeVisible();
        await page.getByRole("button", { name: "Open navigation menu" }).click();
        await expect(navigationMenu, "Closing mobile navigation hides the navigation menu").toBeHidden();
      } else {
        await expect(
          page.getByRole("link", { name: "Home", exact: true }),
          "Desktop navigation displays the Home link",
        ).toBeVisible();
      }
    }, { box: true });
  });

  test("Quick play redirects anonymous visitors to sign-in", async ({ page }) => {
    await page.goto("/");
    await expectHydrated(page);
    await test.step("Select quick play", async () => {
      await page.getByRole("button", { name: "1+2", exact: true }).click();
      await expect(page, "Selecting a quick-play time control takes visitors to the sign-in page").toHaveURL(
        /\/login$/,
      );
      await expect(
        page.getByRole("heading", { name: "Sign in" }),
        "The sign-in page displays its Sign in heading",
      ).toBeVisible();
    }, { box: true });
  });
});
