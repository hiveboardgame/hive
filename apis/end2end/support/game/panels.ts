import { expect, type Page } from "@playwright/test";

export type GameTab = "Game" | "History" | "Chat";

export async function showTab(page: Page, tab: GameTab, isMobileLayout: boolean) {
  const mobileChat = page.getByRole("button", { name: "Open chat", exact: true });
  if (isMobileLayout) {
    await expect(mobileChat).toBeVisible();
    const chatOpen = await mobileChat.getAttribute("aria-expanded") === "true";
    if (chatOpen !== (tab === "Chat")) await mobileChat.click();
    await expect(mobileChat).toHaveAttribute("aria-expanded", String(tab === "Chat"));
    if (tab === "Chat") {
      await expect(page.getByRole("dialog", { name: "Chat messages" })).toBeVisible();
      await expect(page.getByLabel("Chat message", { exact: true })).toBeVisible();
      return;
    }
    if (tab === "Game") {
      await expect(page.getByRole("dialog", { name: "Chat messages" })).toBeHidden();
      return;
    }

    throw new Error("Portrait history uses the move navigation buttons, not a History tab");
  }
  await page.getByText(tab, { exact: true }).click();
  if (tab === "Chat") {
    await expect(page.getByLabel("Chat message", { exact: true })).toBeVisible();
  }
}
