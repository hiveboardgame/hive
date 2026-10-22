import { expect, type Page } from "@playwright/test";
import type { Player } from "../browser/player";
import { expectHydrated } from "../browser/hydration";

export type ChallengeColor = "White" | "Black" | "Random";

// The accepting player must be on the home page before the challenge arrives.
export async function openChallengeHome({ page, username }: Player) {
  if (new URL(page.url()).pathname !== "/") await page.goto("/");
  await expectHydrated(page);
  await expect(page.getByRole("button", { name: username, exact: true })).toBeVisible();
}

export async function createDirectChallenge(
  { page, username }: Player,
  opponent: Player,
  color: ChallengeColor,
) {
  await openChallengeHome(opponent);
  await page.goto(`/@/${opponent.username}`);
  await expectHydrated(page);
  await expect(page.getByRole("button", { name: username, exact: true })).toBeVisible();
  const challenge = page.getByTitle("Challenge to a game");
  await expect(challenge).toBeVisible();
  await challenge.click();
  await expect(page.getByText(`Opponent: ${opponent.username}`)).toBeVisible();
  await page.getByTitle(color, { exact: true }).click();
}

// These scenarios create one outstanding challenge per challenger at a time.
export function challengeFrom(page: Page, username: string) {
  return page
    .getByRole("row")
    .filter({ has: page.getByRole("link", { name: username, exact: true }) });
}

export async function acceptChallenge({ challenger, opponent }: {
  challenger: Player;
  opponent: Player;
}) {
  const challenge = challengeFrom(opponent.page, challenger.username);
  await expect(challenge).toBeVisible();
  await challenge.getByRole("button", { name: "Accept Challenge", exact: true }).click();
  await expect(challenge, "Acceptance removes the pending challenge").toHaveCount(0);
  await expect(challenger.page).toHaveURL(/\/game\//);
  await expect(opponent.page).toHaveURL(challenger.page.url());
}
