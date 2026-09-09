import { expect, type Page } from "@playwright/test";
import { dismissOnboardingBanners } from "./onboarding";
import { stripSecureCookiesForWebKit } from "./session_cookies";
import { expectHydrated } from "./hydration";

export async function signIn(page: Page, username: string, applicationURL: string) {
  await dismissOnboardingBanners(page);
  // WebKit refuses to store Secure cookies served over HTTP, even on localhost.
  await stripSecureCookiesForWebKit(
    page.context(),
    applicationURL,
  );
  await page.goto("/login");
  await expectHydrated(page);
  await page.getByLabel("Email").fill(`${username}@example.test`);
  await page.getByLabel("Password").fill("password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("button", { name: username, exact: true })).toBeVisible();
  // A login response can update the UI even if the browser rejects its cookie.
  await page.goto("/");
  await expectHydrated(page);
  await expect(
    page.getByRole("button", { name: username, exact: true }),
    "The authenticated session survives navigation to home",
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Create a game", exact: true })).toBeVisible();
}
