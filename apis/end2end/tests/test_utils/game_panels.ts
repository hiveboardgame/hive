import { expect, type Locator, type Page } from "playwright/test";

export type GameTab = "Game" | "History" | "Chat";

export function historyControl(page: Page, direction: "First" | "Previous" | "Next" | "Last"): Locator {
  return page.getByRole("button", { name: `${direction} move`, exact: true });
}

export function chatControl(page: Page) {
  return page.getByText("Chat", { exact: true })
    .or(page.getByRole("button", { name: "Open chat", exact: true }));
}

export async function showTab(page: Page, tab: GameTab) {
  const mobileChat = page.getByRole("button", { name: "Open chat", exact: true });
  if (await mobileChat.isVisible()) {
    const chatOpen = await mobileChat.getAttribute("aria-expanded") === "true";
    if (chatOpen !== (tab === "Chat")) await mobileChat.click();
    if (tab === "Chat") {
      await expect(page.getByRole("dialog", { name: "Chat messages" })).toBeVisible();
      return;
    }
    if (tab === "Game") {
      await expect(page.getByRole("dialog", { name: "Chat messages" })).toBeHidden();
      return;
    }

    throw new Error("Portrait history uses the move navigation buttons, not a History tab");
  }
  await page.getByText(tab, { exact: true }).click();
}

export async function reviewHistory(
  page: Page,
  expectedMoveCount: number,
  expectedMoves: ReadonlyArray<readonly [number, string]> = [],
) {
  await showTab(page, "History");
  const moves = page.getByText(/^\d+\. [wb][A-Z]/);
  await expect(moves).toHaveCount(expectedMoveCount);
  for (const [index, notation] of expectedMoves) {
    await expect(moves.nth(index)).toContainText(notation);
  }
}
