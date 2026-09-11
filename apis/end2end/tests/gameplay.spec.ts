import { expect, test } from "../support/fixtures";
import { boardPiece, expectPieceAt, movePiece, placePiece } from "../support/game/board";
import { confirmControl } from "../support/game/controls";
import { chatControl, historyControl, reviewHistory, showTab } from "../support/game/panels";
import { startGame } from "../support/game/setup";

const boardPositions = {
  openingAnt: "16, 16",
  blackAnt: "15, 16",
  whiteQueen: "16, 17",
  blackQueen: "15, 15",
  whiteBeetle: "17, 16",
  blackGrasshopper: "16, 14",
  beetleMove: "16, 17",
};

test.describe("Gameplay", () => {
  test.describe.configure({ timeout: 90_000 });

  test("History, takebacks, and agreed draws after seven turns", async ({
    players,
    isMobileLayout,
  }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    const piecesOnBoard = whitePage.getByRole("button", { name: / on board$/ });
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });

    await test.step("Play the opening and review history", async () => {
      await test.step("Place both opening ants", async () => {
        await placePiece(whitePage, "White Ant 1", boardPositions.openingAnt);
        await expect(boardPiece(whitePage, "White Ant 1")).toBeVisible();
        await placePiece(blackPage, "Black Ant 1", boardPositions.blackAnt);
      }, { box: true });

      await test.step("Review the opening moves", async () => {
        if (isMobileLayout) {
          await expectPieceAt(whitePage, "Black Ant 1", boardPositions.blackAnt);
          await historyControl(whitePage, "Previous").click();
          await expect(piecesOnBoard).toHaveCount(1);
          await expectPieceAt(whitePage, "White Ant 1", boardPositions.openingAnt);
          await expect(boardPiece(whitePage, "Black Ant 1")).toHaveCount(0);
          await expect(historyControl(whitePage, "Previous")).toBeDisabled();
          await historyControl(whitePage, "Next").click();
          await expectPieceAt(whitePage, "Black Ant 1", boardPositions.blackAnt);
          await expect(piecesOnBoard).toHaveCount(2);
          await expect(historyControl(whitePage, "Next")).toBeDisabled();
        } else {
          await reviewHistory(whitePage, 2);
        }
        await showTab(whitePage, "Game");
      }, { box: true });
    }, { box: true });

    await test.step("Play through turn seven and verify history", async () => {
      await test.step("Play turns three through five", async () => {
        await placePiece(whitePage, "White Queen", boardPositions.whiteQueen);
        await placePiece(blackPage, "Black Queen", boardPositions.blackQueen);
        await placePiece(whitePage, "White Beetle 1", boardPositions.whiteBeetle);
      }, { box: true });

      await test.step("Cannot move during the opponent's turn", async () => {
        await boardPiece(whitePage, "White Beetle 1").click();
        await expect(whitePage.getByRole("button", { name: /^Move to board position / })).toHaveCount(0);
        await whitePage.getByRole("button", { name: "White Ant 2 in reserve", exact: true }).click();
        await expect(whitePage.getByRole("button", { name: /^Move to board position / })).toHaveCount(0);
      }, { box: true });

      await test.step("Play turns six and seven", async () => {
        await placePiece(blackPage, "Black Grasshopper 1", boardPositions.blackGrasshopper);
        await movePiece(whitePage, "White Beetle 1", boardPositions.beetleMove);
      }, { box: true });

      await test.step("Verify the seven-move history", async () => {
        if (isMobileLayout) {
          await test.step("Step back and see the beetle return beside the queen", async () => {
            await expectPieceAt(whitePage, "White Beetle 1", boardPositions.beetleMove, 1);
            await historyControl(whitePage, "Previous").click();
            await expectPieceAt(whitePage, "White Beetle 1", boardPositions.whiteBeetle);
            await expectPieceAt(whitePage, "White Queen", boardPositions.whiteQueen);
          }, { box: true });

          await test.step("Jump to the first move and advance the board one move", async () => {
            await historyControl(whitePage, "First").click();
            await expect(piecesOnBoard).toHaveCount(1);
            await expectPieceAt(whitePage, "White Ant 1", boardPositions.openingAnt);
            await expect(historyControl(whitePage, "First")).toBeDisabled();
            await historyControl(whitePage, "Next").click();
            await expect(piecesOnBoard).toHaveCount(2);
            await expectPieceAt(whitePage, "Black Ant 1", boardPositions.blackAnt);
          }, { box: true });

          await test.step("Return to the latest board with the beetle on the queen", async () => {
            await historyControl(whitePage, "Last").click();
            await expectPieceAt(whitePage, "White Beetle 1", boardPositions.beetleMove, 1);
            await expectPieceAt(whitePage, "Black Grasshopper 1", boardPositions.blackGrasshopper);
            await expect(historyControl(whitePage, "Last")).toBeDisabled();
          }, { box: true });
        } else {
          await reviewHistory(whitePage, 7, [
            [3, "bQ"],
            [6, "wB1"],
          ]);
        }
        await showTab(whitePage, "Game");
      }, { box: true });
    }, { box: true });

    await test.step("Negotiate rejected and accepted takebacks", async () => {
      await test.step("Reject a takeback request", async () => {
        await confirmControl(blackPage, "Request Takeback");
        await expect(whitePage.getByText("Opponent wants a takeback")).toBeVisible();
        await whitePage.getByTitle("Reject Takeback").click();
        if (isMobileLayout) {
          await expectPieceAt(whitePage, "White Beetle 1", boardPositions.beetleMove, 1);
        } else {
          await reviewHistory(whitePage, 7);
        }
        await showTab(whitePage, "Game");
      }, { box: true });

      await test.step("Accept a takeback request", async () => {
        await confirmControl(blackPage, "Request Takeback");
        await expect(whitePage.getByText("Opponent wants a takeback")).toBeVisible();
        await whitePage.getByTitle("Accept Takeback").click();
        if (isMobileLayout) {
          for (const page of [whitePage, blackPage]) {
            await expectPieceAt(page, "White Beetle 1", boardPositions.whiteBeetle);
            await expectPieceAt(page, "Black Grasshopper 1", boardPositions.blackGrasshopper);
          }
        } else {
          await reviewHistory(whitePage, 6, [[5, "bG1"]]);
        }
      }, { box: true });
    }, { box: true });

    await test.step("Negotiate and confirm a draw", async () => {
      await test.step("Reject a draw offer", async () => {
        await showTab(whitePage, "Game");
        await confirmControl(whitePage, "Offer Draw");
        await expect(blackPage.getByText("Opponent offers a draw")).toBeVisible();
        await blackPage.getByTitle("Reject Draw").click();
        await expect(whitePage.getByTitle("Offer Draw")).toBeVisible();
      }, { box: true });

      await test.step("Accept a draw offer and verify the outcome", async () => {
        await confirmControl(whitePage, "Offer Draw");
        await expect(blackPage.getByText("Opponent offers a draw")).toBeVisible();
        await blackPage.getByTitle("Accept Draw").click();
        for (const page of [whitePage, blackPage]) {
          if (!isMobileLayout) await showTab(page, "History");
          await expect(page.getByText(isMobileLayout ? "½-½ Draw agreed" : "Draw agreed", { exact: true })).toBeVisible();
        }
      }, { box: true });
    }, { box: true });
  });

  test("Chat, unread indicators, and resignation after four turns", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });

    await test.step("Play two moves each", async () => {
      await placePiece(whitePage, "White Ant 1", boardPositions.openingAnt);
      await placePiece(blackPage, "Black Ant 1", boardPositions.blackAnt);
      await placePiece(whitePage, "White Queen", boardPositions.whiteQueen);
      await placePiece(blackPage, "Black Queen", boardPositions.blackQueen);
    }, { box: true });

    await test.step("Exchange chat messages", async () => {
      const firstMessage = `Hello from ${players.userOne.username}`;
      const secondMessage = `Hello from ${players.userTwo.username}`;
      const userOneChatControl = chatControl(whitePage);
      const userTwoChatControl = chatControl(blackPage);
      const unreadAlert = /ui-button-danger|ui-header-action-alert/;

      await test.step("Send a message to a player outside chat", async () => {
        await showTab(whitePage, "Chat");
        const chatInput = whitePage.getByLabel("Chat message", { exact: true });
        await chatInput.fill(firstMessage);
        await chatInput.press("Enter");
        await expect(whitePage.getByRole("log", { name: "Chat messages" })).toContainText(firstMessage);
      }, { box: true });

      await test.step("Show a red alert for the unread message", async () => {
        await expect(userTwoChatControl).toHaveClass(unreadAlert);
      }, { box: true });

      await test.step("Read the message and reply from chat", async () => {
        await showTab(blackPage, "Chat");
        await expect(blackPage.getByRole("log", { name: "Chat messages" })).toContainText(firstMessage);
        const chatInput = blackPage.getByLabel("Chat message", { exact: true });
        await chatInput.fill(secondMessage);
        await chatInput.press("Enter");
        await expect(blackPage.getByRole("log", { name: "Chat messages" })).toContainText(secondMessage);
      }, { box: true });

      await test.step("Receive the reply while chat is open", async () => {
        await expect(whitePage.getByRole("log", { name: "Chat messages" })).toContainText(secondMessage);
      }, { box: true });

      await test.step("Keep the chat control clear while the message is read", async () => {
        await expect(userOneChatControl).not.toHaveClass(unreadAlert);
      }, { box: true });
    }, { box: true });

    await test.step("Resign and confirm the outcome", async () => {
      await showTab(whitePage, "Game");
      await showTab(blackPage, "Game");
      await confirmControl(whitePage, "Resign");
      for (const player of [whitePage, blackPage]) {
        await expect(
          player.getByText(`${players.userTwo.username} won by resignation`, { exact: true }),
        ).toBeVisible();
      }
    }, { box: true });
  });
});
