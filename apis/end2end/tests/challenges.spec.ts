import { BrowserContext, expect, Page, test } from "playwright/test";

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:3000";

async function signIn(page: Page, username: string) {
  await page.goto("/login");
  await page.getByLabel("Email").fill(`${username}@example.test`);
  await page.getByLabel("Password").fill("password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("link", { name: "Login" })).toBeHidden();
  await expect(page.getByRole("button", { name: username })).toBeVisible();
}

function challengeFrom(page: Page, username: string) {
  return page
    .getByRole("row")
    .filter({ has: page.getByRole("link", { name: username, exact: true }) });
}

async function createDirectChallenge(page: Page) {
  await page.goto("/@/user_2");
  await page.getByTitle("Challenge to a game").click();
  await expect(page.getByText("Opponent: user_2")).toBeVisible();
  await page.getByTitle("Random").click();
}

test("two users can decline, cancel, accept, and abort challenges", async ({ browser, page }, testInfo) => {
  test.setTimeout(60_000);

  const viewport = page.viewportSize();
  let userTwoContext: BrowserContext | undefined;

  try {
    const userTwo = await test.step("Sign in both players", async () => {
      userTwoContext = await browser.newContext({
        baseURL,
        ...(viewport ? { viewport } : {}),
      });
      const userTwo = await userTwoContext.newPage();
      await signIn(page, "user_1");
      await signIn(userTwo, "user_2");
      return userTwo;
    }, { box: true });

    await test.step("Decline a direct challenge", async () => {
      const directChallenge = await test.step("Create the direct challenge", async () => {
        await createDirectChallenge(page);
        const directChallenge = challengeFrom(userTwo, "user_1");
        await expect(directChallenge).toBeVisible();
        return directChallenge;
      }, { box: true });

      await test.step("Decline it and confirm removal", async () => {
        await directChallenge.getByRole("button", { name: "Decline Challenge" }).click();
        await expect(directChallenge).toHaveCount(0);
      }, { box: true });
    }, { box: true });

    await test.step("Create and cancel a public challenge", async () => {
      const publicChallenge = await test.step("Create the public challenge", async () => {
        await page.goto("/");
        await page.getByRole("button", { name: "1+2", exact: true }).click();
        const publicChallenge = challengeFrom(userTwo, "user_1");
        await expect(publicChallenge).toBeVisible();
        return publicChallenge;
      }, { box: true });

      await test.step("Verify the opponent cannot cancel it", async () => {
        await expect(
          publicChallenge.getByRole("button", { name: "Cancel Challenge" }),
        ).toHaveCount(0);
      }, { box: true });

      await test.step("Cancel it and confirm removal", async () => {
        const ownPublicChallenge = challengeFrom(page, "user_1");
        await ownPublicChallenge.getByRole("button", { name: "Cancel Challenge" }).click();
        await expect(publicChallenge).toHaveCount(0);
      }, { box: true });
    }, { box: true });

    await test.step("Accept a direct challenge and abort the game", async () => {
      await test.step("Create and accept the direct challenge", async () => {
        await createDirectChallenge(page);
        const acceptedChallenge = challengeFrom(userTwo, "user_1");
        await expect(acceptedChallenge).toBeVisible();
        await acceptedChallenge.getByRole("button", { name: "Accept Challenge" }).click();
        await expect(page).toHaveURL(/\/game\//);
        await expect(userTwo).toHaveURL(/\/game\//);
      }, { box: true });

      await test.step("Abort the game", async () => {
        if (testInfo.project.name.endsWith("-mobile")) {
          await page.getByTitle("Show controls").click();
        }
        const abort = page.getByTitle("Abort");
        await expect(abort).toBeVisible();
        await abort.click();
        await abort.click();
      }, { box: true });

      await test.step("Confirm the abort outcome", async () => {
        await expect(page.getByRole("alert")).toContainText("user_1 aborted the game");
        await expect(userTwo.getByRole("alert")).toContainText("user_1 aborted the game");
        await expect(page).toHaveURL(/\/$/);
        await expect(userTwo).toHaveURL(/\/$/);
      }, { box: true });
    }, { box: true });
  } finally {
    await userTwoContext?.close();
  }
});
