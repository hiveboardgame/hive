import { expect, type Page } from "@playwright/test";
import { dismissOnboardingBanners } from "./onboarding";
import { stripSecureCookiesForWebKit } from "./session_cookies";

export async function signIn(page: Page, username: string, applicationURL: string) {
  await dismissOnboardingBanners(page);
  // WebKit refuses to store Secure cookies served over HTTP, even on localhost.
  await stripSecureCookiesForWebKit(
    page.context(),
    applicationURL,
  );
  await page.goto("/login");
  await page.getByLabel("Email").fill(`${username}@example.test`);
  await page.getByLabel("Password").fill("password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("button", { name: username })).toBeVisible();
  // A login response can update the UI even if the browser rejects its cookie.
  await page.reload();
  await expect(
    page.getByRole("button", { name: username }),
    "The authenticated session survives a full page reload",
  ).toBeVisible();
}
