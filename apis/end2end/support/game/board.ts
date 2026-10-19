import { expect, type Locator, type Page } from "@playwright/test";

export function boardPiece(page: Page, pieceName: string): Locator {
  return board(page).getByRole("button", { name: `${pieceName} on board`, exact: true });
}

export async function expectPieceAt(page: Page, pieceName: string, position: string, level = 0) {
  const piece = boardPiece(page, pieceName);
  await expect(piece).toBeVisible();
  await expect(piece).toHaveAttribute("data-position", position);
  await expect(piece).toHaveAttribute("data-stack-level", String(level));
}

async function clickBoardPosition(page: Page, pieceName: string, position: string) {
  await board(page)
    .getByRole("button", {
      name: `Move to board position ${position}`,
      exact: true,
    })
    .click();
  await board(page).getByRole("button", { name: `${pieceName} move preview`, exact: true }).click();
}

export async function placePiece(page: Page, pieceName: string, position: string) {
  await page.getByRole("button", { name: `${pieceName} in reserve`, exact: true }).click();
  await clickBoardPosition(page, pieceName, position);
}

export async function movePiece(page: Page, pieceName: string, position: string) {
  await boardPiece(page, pieceName).click();
  await clickBoardPosition(page, pieceName, position);
}

export function board(page: Page) { return page.getByTestId("hive-board"); }

export async function boardViewport(page: Page) {
  return board(page).evaluate(element => {
    const { x, y, width, height } = (element as SVGSVGElement).viewBox.baseVal;
    return { origin: [x, y], size: [width, height] };
  });
}

export function reservePiece(page: Page, name: string) {
  return page.getByRole("button", { name: `${name} in reserve`, exact: true });
}

export function targets(page: Page) {
  return board(page).getByRole("button", { name: /^Move to board position / });
}

export function targetAt(page: Page, position: string) {
  return board(page).getByRole("button", { name: `Move to board position ${position}`, exact: true });
}

export async function targetPosition(target: Locator) {
  const q = await target.getAttribute("data-hg-target-q");
  const r = await target.getAttribute("data-hg-target-r");
  if (q === null || r === null) throw new Error("The board target has no position");
  return `${q}, ${r}`;
}

export async function boardSnapshot(page: Page) {
  return board(page).getByRole("button", { name: / on board$/ }).evaluateAll(pieces => pieces.map(piece => ({
    name: piece.getAttribute("aria-label"),
    position: piece.getAttribute("data-position"),
    level: piece.getAttribute("data-stack-level"),
  })).sort((a, b) => (a.name ?? "").localeCompare(b.name ?? "")));
}

export async function pieceSeparation(first: Locator, second: Locator) {
  const [a, b] = await Promise.all([first.boundingBox(), second.boundingBox()]);
  if (!a || !b) throw new Error("The stack pieces have no rendered bounds");
  return Math.hypot(a.x + a.width / 2 - b.x - b.width / 2, a.y + a.height / 2 - b.y - b.height / 2);
}
