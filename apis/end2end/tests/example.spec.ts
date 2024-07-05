import { test, expect } from "@playwright/test";

test("Positive and negative login test", async ({ page, browser }) => {
  await page.goto("/login");
  //Negative
  await page.getByPlaceholder("E-mail").fill("TestUser1Email@example.com");
  await page.getByLabel("Password").fill("Wrong Password!");
  await page.getByRole("button", { name: "Sign In" }).click();

  await expect(page).toHaveURL("/login");
  await expect(page.getByText(/Invalid email or password/)).toBeVisible();
  //Positive

  await page.getByPlaceholder("E-mail").fill("TestUser1Email@example.com"); // TODO: change to another user
  await page.getByLabel("Password").fill("12345678");
  await page.getByRole("button", { name: "Sign In" }).click();

  await expect(page).toHaveURL("/");
  await expect(page.getByText(/online player/)).toContainText("TestUser1");
});

test('Search user', async ({ page }) => {
  page.on('websocket', ws => {
    console.log(`WebSocket opened: ${ws.url()}>`);
  });
  await page.goto('/');
  await page.getByPlaceholder('Search players').fill('TestUser1');
  await expect(page.getByRole('link', { name: 'TestUser1' })).toBeVisible();
});
