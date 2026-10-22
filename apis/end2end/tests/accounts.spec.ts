import { expect, test } from "../support/fixtures";
import { expectHydrated, reloadHydrated } from "../support/browser/hydration";

// Full session journeys include deliberate document reloads of the native WASM app.
test.describe.configure({ timeout: 90_000 });

test("registration authenticates immediately, logout revokes access, and login restores it @journey", async ({ data, page }) => {
  const account = data.registration();
  await page.goto("/register");
  await expectHydrated(page);
  await page.getByRole("textbox", { name: /^Username/ }).fill(account.username);
  await page.getByRole("textbox", { name: /^Email/ }).fill(account.email);
  await page.locator('input[name="password"]').fill(account.password);
  await page.getByLabel("Confirm password", { exact: true }).fill(account.password);
  await page.getByRole("checkbox").check();
  await page.getByRole("button", { name: "Sign up", exact: true }).click();
  const menu = page.getByRole("button", { name: account.username, exact: true });
  await expect(menu).toBeVisible();

  // A private challenge exercises the new session without refresh or cross-test matchmaking.
  await page.getByRole("button", { name: "Custom", exact: true }).click();
  const dialog = page.getByRole("dialog");
  const publicChallenge = dialog.locator("div").filter({ hasText: /^Private\s*Public$/ }).getByRole("checkbox");
  if (await publicChallenge.isChecked()) await publicChallenge.locator("..").click();
  await expect(publicChallenge).not.toBeChecked();
  await dialog.getByTitle("White", { exact: true }).click();
  const cancel = page.getByRole("button", { name: "Cancel Challenge", exact: true });
  await cancel.click();
  await expect(cancel).toHaveCount(0);
  await menu.click();
  await page.getByRole("button", { name: "Logout", exact: true }).click();
  await expect(menu).toHaveCount(0);
  await page.goto("/account");
  await expect(page).toHaveURL(/\/login(?:\?|$)/);
  await expectHydrated(page);
  await page.getByLabel("Email or username", { exact: true }).fill(account.email);
  await page.locator('input[name="password"]').fill(account.password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(menu).toBeVisible();
  await page.goto("/account");
  await expectHydrated(page);
  await expect(page.getByRole("button", { name: "Delete Account", exact: true })).toBeVisible();
});

test("deletion checks the password and revokes the account persistently @journey", async ({ data, player }) => {
  const { users } = await data.users(1);
  const account = users[0];
  const { page } = await player(account, { path: "/account" });
  await expectHydrated(page);
  const form = page.locator("form").filter({ has: page.getByRole("button", { name: "Delete Account", exact: true }) });
  const password = form.getByLabel("Current password", { exact: true });
  await password.fill("wrong-password");
  await form.getByRole("button", { name: "Delete Account", exact: true }).click();
  await expect(form.getByText(/Could not delete account/)).toBeVisible();
  await reloadHydrated(page);
  await expect(page.getByRole("button", { name: account.username, exact: true })).toBeVisible();
  await password.fill(account.password);
  await form.getByRole("button", { name: "Delete Account", exact: true }).click();
  await expect(page).toHaveURL(/\/$/);
  await reloadHydrated(page);
  await page.goto("/account");
  await expect(page).toHaveURL(/\/login(?:\?|$)/);
  await expectHydrated(page);
  await page.getByLabel("Email or username", { exact: true }).fill(account.email);
  await page.locator('input[name="password"]').fill(account.password);
  const action = await page.locator("form").filter({ has: page.locator('input[name="email"]') })
    .evaluate(form => (form as HTMLFormElement).action);
  const submitted = page.waitForResponse(response => response.url() === action && response.request().method() === "POST");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await submitted;
  await page.goto("/account");
  await expect(page).toHaveURL(/\/login(?:\?|$)/);
  await expectHydrated(page);
  await expect(page.getByRole("button", { name: account.username, exact: true })).toHaveCount(0);
});
