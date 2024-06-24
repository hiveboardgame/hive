import { test, expect } from "@playwright/test";

test("Invalid login", async ({ page, browser }) => {
  await page.goto("/login");

  await page.getByPlaceholder("E-mail").fill("TestUser1Email@example.com");
  await page.getByLabel("Password").fill("Wrong Password!");
  await page.getByRole("button", { name: "Sign In" }).click();

  await expect(page).toHaveURL("/login");
  await expect(page.getByText(/Invalid email or password/)).toBeVisible();
});


test("Correct login and online indicator", async ({ page, browser }) => {
  await page.goto("/login");

  await page.getByPlaceholder("E-mail").fill("TestUser1Email@example.com");
  await page.getByLabel("Password").fill("12345678");
  await page.getByRole("button", { name: "Sign In" }).click();

  await expect(page).toHaveURL("/");
  await expect(page.getByText(/Online players:/)).toContainText("TestUser1");
});
