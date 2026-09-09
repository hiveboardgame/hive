import { expect, test } from "../../support/fixtures";
import { chatControl, showTab } from "../../support/game/panels";
import { startGame } from "../../support/game/setup";

test.describe("Chat", () => {
  test.describe.configure({ timeout: 90_000 });

  test("Exchange messages while both players have chat open", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await test.step("Exchange messages while both players have chat open", async () => {
      await showTab(whitePage, "Chat", isMobileLayout);
      await showTab(blackPage, "Chat", isMobileLayout);
      for (const [sender, recipient] of [[players.userOne, players.userTwo], [players.userTwo, players.userOne]]) {
        await test.step(`Send a message from ${sender.username}`, async () => {
          const message = `Hello from ${sender.username}`;
          const input = sender.page.getByLabel("Chat message", { exact: true });
          await input.fill(message);
          await input.press("Enter");
          for (const page of [sender.page, recipient.page]) {
            await expect(page.getByRole("log", { name: "Chat messages" })).toContainText(message);
          }
          await expect(chatControl(recipient.page)).not.toHaveClass(/ui-button-danger|ui-header-action-alert/);
        }, { box: true });
      }
    }, { box: true });
  });

  test("Show an unread alert and clear it when the message is read", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await test.step("Show an unread alert and clear it when the message is read", async () => {
      await showTab(blackPage, "Game", isMobileLayout);
      await showTab(whitePage, "Chat", isMobileLayout);
      const message = `Hello from ${players.userOne.username}`;
      const input = whitePage.getByLabel("Chat message", { exact: true });
      await input.fill(message);
      await input.press("Enter");
      await expect(whitePage.getByRole("log", { name: "Chat messages" })).toContainText(message);
      await expect(chatControl(blackPage)).toHaveClass(/ui-button-danger|ui-header-action-alert/);
      await showTab(blackPage, "Chat", isMobileLayout);
      await expect(blackPage.getByRole("log", { name: "Chat messages" })).toContainText(message);
      await expect(chatControl(blackPage)).not.toHaveClass(/ui-button-danger|ui-header-action-alert/);
    }, { box: true });
  });
});
