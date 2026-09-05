import { expect, Page } from "playwright/test";

export async function signIn(page: Page, username: string) {
  await page.goto("/login");
  await page.getByLabel("Email").fill(`${username}@example.test`);
  await page.getByLabel("Password").fill("password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("button", { name: username })).toBeVisible();
}

export async function createTargetedWhiteChallenge(page: Page, username: string) {
  await page.goto(`/@/${username}`);
  await page.getByTitle("Challenge to a game").click();
  await expect(page.getByText(`Opponent: ${username}`)).toBeVisible();
  await page.getByTitle("White").click();
}

export function challengeFrom(page: Page, username: string) {
  return page
    .getByRole("row")
    .filter({ has: page.getByRole("link", { name: username, exact: true }) });
}

async function clickBoardPosition(page: Page, pieceName: string, position: string) {
  await page
    .getByRole("button", {
      name: `Move to board position ${position}`,
      exact: true,
    })
    .click();
  await page.getByRole("button", { name: `${pieceName} move preview`, exact: true }).click();
}

export async function placePiece(page: Page, pieceName: string, position: string) {
  await page.getByRole("button", { name: `${pieceName} in reserve`, exact: true }).click();
  await clickBoardPosition(page, pieceName, position);
}

export async function movePiece(page: Page, pieceName: string, position: string) {
  await page.getByRole("button", { name: `${pieceName} on board`, exact: true }).click();
  await clickBoardPosition(page, pieceName, position);
}

export async function confirmControl(page: Page, title: string) {
  const control = page.getByTitle(title);
  await control.click();
  await control.click();
}

export async function showControlsIfMobile(page: Page, isMobile: boolean) {
  if (isMobile) {
    await page.getByTitle("Show controls").click();
  }
}

function historyMoves(page: Page) {
  return page.getByText(/^\d+\. [wb][A-Z]/);
}

export type GameTab = "Game" | "History" | "Chat";

export async function showTab(page: Page, tab: GameTab) {
  await page.getByText(tab, { exact: true }).click();
}

export async function reviewHistory(
  page: Page,
  expectedMoveCount: number,
  expectedMoves: ReadonlyArray<readonly [number, string]> = [],
) {
  await showTab(page, "History");
  const moves = historyMoves(page);
  await expect(moves).toHaveCount(expectedMoveCount);
  for (const [index, notation] of expectedMoves) {
    await expect(moves.nth(index)).toContainText(notation);
  }
}
