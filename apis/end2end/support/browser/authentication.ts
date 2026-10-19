import { expect, type Page } from "@playwright/test";
import type { Account } from "../data";
import { expectHydrated } from "./hydration";

export async function signIn(page: Page, account: Account, path = "/") {
  await page.goto("/login");
  const form = page.locator("form").filter({ has: page.locator('input[name="email"]') });
  const submission = await form.evaluate(element => {
    const form = element as HTMLFormElement;
    return { action: form.action, fields: Object.fromEntries(new FormData(form).entries()) as Record<string, string> };
  });
  const response = await page.request.post(submission.action, {
    form: { ...submission.fields, email: account.email, password: account.password, pathname: "/" },
    headers: { Origin: new URL(page.url()).origin },
  });
  expect(response.ok(), "The real login action accepts the fixture credentials").toBeTruthy();
  await page.goto(path);
  await expectHydrated(page);
  await expect(page.getByRole("button", { name: account.username, exact: true })).toBeVisible();
}
