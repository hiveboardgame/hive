import { expect, type Page } from "playwright/test";
import type { Player } from "./player";

export type ChallengeColor = "White" | "Black" | "Random";

export async function createDirectChallenge(
  page: Page,
  opponentUsername: string,
  color: ChallengeColor,
) {
  await page.goto(`/@/${opponentUsername}`);
  await expect(page.getByRole("link", { name: "Login", exact: true })).toBeHidden();
  const challenge = page.getByTitle("Challenge to a game");
  await expect(challenge).toBeVisible();
  await challenge.click();
  await expect(page.getByText(`Opponent: ${opponentUsername}`)).toBeVisible();
  await page.getByTitle(color, { exact: true }).click();
}

export async function createPublicChallenge(page: Page, timeControl: string) {
  await page.goto("/");
  await page.getByRole("button", { name: timeControl, exact: true }).click();
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
  await expect(challenger.page).toHaveURL(/\/game\//);
  await expect(opponent.page).toHaveURL(challenger.page.url());
}

export async function declineChallenge(page: Page, challengerUsername: string) {
  await challengeFrom(page, challengerUsername)
    .getByRole("button", { name: "Decline Challenge", exact: true }).click();
}

export async function cancelChallenge(page: Page, creatorUsername: string) {
  await challengeFrom(page, creatorUsername)
    .getByRole("button", { name: "Cancel Challenge", exact: true }).click();
}
