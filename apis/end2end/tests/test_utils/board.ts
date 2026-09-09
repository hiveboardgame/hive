import { expect, type Locator, type Page } from "playwright/test";

export function boardPiece(page: Page, pieceName: string): Locator {
  return page.getByRole("button", { name: `${pieceName} on board`, exact: true });
}

export async function expectPieceAt(page: Page, pieceName: string, position: string, level = 0) {
  const piece = boardPiece(page, pieceName);
  await expect(piece).toBeVisible();
  await expect(piece).toHaveAttribute("data-position", position);
  await expect(piece).toHaveAttribute("data-stack-level", String(level));
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
  await boardPiece(page, pieceName).click();
  await clickBoardPosition(page, pieceName, position);
}
