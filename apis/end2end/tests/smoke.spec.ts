import { expect, test } from "playwright/test";

test("anonymous visitors can load and use the home page", async ({ page }, testInfo) => {
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
  // The SSR shell initially marks main as hidden and reveals it during
  // hydration. The development WASM bundle is large, so allow the complete
  // download and compilation before asserting the app.
  const main = page.locator("main");
  await expect(
    main,
    "The hydrated home page removes the hidden state from its main content",
  ).not.toHaveClass(/(?:^|\s)hidden(?:\s|$)/, { timeout: 45 * 1000 });
  await expect(main, "The hydrated home page shows its main content").toBeVisible();
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

  await page.getByRole("button", { name: "1+2", exact: true }).click();
  await expect(page, "Selecting a quick-play time control takes visitors to the sign-in page").toHaveURL(
    /\/login$/,
  );
  await expect(
    page.getByRole("heading", { name: "Sign in" }),
    "The sign-in page displays its Sign in heading",
  ).toBeVisible();
});
