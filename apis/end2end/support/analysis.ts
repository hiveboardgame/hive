import { board, expectPieceAt, reservePiece, targets, targetPosition } from "./game/board";
import { boardPositions, openingFile } from "./game/opening";
import { expect, type Page } from "@playwright/test";
import { expectHydrated } from "./browser/hydration";
import { activate } from "./gestures";

export async function openAnalysis(page: Page) {
  await page.goto("/analysis");
  await expectHydrated(page);
  await expect(board(page)).toBeVisible();
  await expect(reservePiece(page, "White Ant 1")).toBeVisible();
}

export async function loadOpening(page: Page) {
  await openAnalysis(page);
  await uploadOpening(page);
}

export async function uploadOpening(page: Page) {
  await expect(board(page)).toBeVisible();
  // Playwright supplies a genuine File to the application's existing upload control.
  await page.locator("#load-analysis").setInputFiles(openingFile);
  await expectPieceAt(page, "White Beetle 1", boardPositions.whiteQueen, 1);
  await board(page).scrollIntoViewIfNeeded();
}

export async function currentNode(page: Page) {
  const row = page.locator('[data-history-current="true"]');
  await expect(row).toHaveCount(1);
  const id = await row.getAttribute("data-history-node-id");
  if (id === null) throw new Error("The selected history row has no identity");
  return id;
}

export async function selectNode(page: Page, id: string, hasTouch: boolean) {
  await activate(page.locator(`[data-history-node-id="${id}"]`), hasTouch);
  await expect(page.locator(`[data-history-node-id="${id}"]`)).toHaveAttribute("data-history-current", "true");
}

export async function expandVariations(page: Page, parentId: string, hasTouch: boolean) {
  const toggle = page.locator(`[data-history-node-id="${parentId}"]`).locator("..").getByRole("button");
  if (await toggle.getAttribute("aria-expanded") !== "true") await activate(toggle, hasTouch);
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
}

export async function placeFromReserve(page: Page, name: string, hasTouch: boolean) {
  await activate(reservePiece(page, name), hasTouch);
  const target = targets(page).first();
  await expect(target).toBeVisible();
  const position = await targetPosition(target);
  await activate(target, hasTouch);
  await expectPieceAt(page, name, position);
  return position;
}

export async function enableDragging(page: Page, username: string) {
  await page.getByRole("button", { name: username, exact: true }).click();
  await page.getByRole("link", { name: "Config", exact: true }).click();
  await expect(page).toHaveURL(/\/config$/);
  const setting = page.getByText("Drag and drop", { exact: true }).locator("..");
  const savedConfig = async () => (await page.context().cookies()).find(cookie => cookie.name === "user_config")?.value;
  const previous = await savedConfig();
  await setting.getByRole("button", { name: "Yes", exact: true }).click();
  await expect.poll(savedConfig).toBeDefined();
  await expect.poll(savedConfig).not.toBe(previous);
}
