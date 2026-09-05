import { BrowserContext, expect, test } from "playwright/test";
import {
  challengeFrom,
  confirmControl,
  createTargetedWhiteChallenge,
  movePiece,
  placePiece,
  reviewHistory,
  showControlsIfMobile,
  showTab,
  signIn,
} from "./navegation_utils";

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:3000";
const boardPositions = {
  openingAnt: "16, 16",
  blackAnt: "15, 16",
  whiteQueen: "16, 17",
  blackQueen: "15, 15",
  whiteBeetle: "17, 16",
  blackGrasshopper: "16, 14",
  beetleMove: "16, 17",
};

test("two players can play seven turns, review history, and negotiate takebacks and draws", async ({
  browser,
  page,
}, testInfo) => {
  test.setTimeout(90_000);

  const viewport = page.viewportSize();
  let userTwoContext: BrowserContext | undefined;

  try {
    const userTwo = await test.step("Sign in and start a game", async () => {
      userTwoContext = await browser.newContext({
        baseURL,
        ...(viewport ? { viewport } : {}),
      });
      const userTwo = await userTwoContext.newPage();

      await test.step("Sign in both players", async () => {
        await signIn(page, "user_1");
        await signIn(userTwo, "user_2");
      }, { box: true });

      await test.step("Create and accept a white challenge", async () => {
        await createTargetedWhiteChallenge(page, "user_2");
        const challenge = challengeFrom(userTwo, "user_1");
        await expect(challenge).toBeVisible();
        await challenge.getByRole("button", { name: "Accept Challenge" }).click();
      }, { box: true });

      await test.step("Confirm both players are ready to play", async () => {
        await expect(page).toHaveURL(/\/game\//);
        await expect(userTwo).toHaveURL(page.url());
        const isMobile = testInfo.project.name.endsWith("-mobile");
        await showControlsIfMobile(page, isMobile);
        await showControlsIfMobile(userTwo, isMobile);
      }, { box: true });
      return userTwo;
    }, { box: true });

    await test.step("Play the opening and review history", async () => {
      await test.step("Place the opening ant", async () => {
        await placePiece(page, "White Ant 1", boardPositions.openingAnt);
        await expect(
          page.getByRole("button", { name: "White Ant 1 on board", exact: true }),
        ).toBeVisible();
      }, { box: true });

      await test.step("Review the opening move", async () => {
        await reviewHistory(page, 1);
        await showTab(page, "Game");
      }, { box: true });
    }, { box: true });

    await test.step("Play through turn seven and verify history", async () => {
      await test.step("Play turns two through five", async () => {
        await placePiece(userTwo, "Black Ant 1", boardPositions.blackAnt);
        await placePiece(page, "White Queen", boardPositions.whiteQueen);
        await placePiece(userTwo, "Black Queen", boardPositions.blackQueen);
        await placePiece(page, "White Beetle 1", boardPositions.whiteBeetle);
      }, { box: true });

      await test.step("Cannot move during the opponent's turn", async () => {
        await page.getByRole("button", { name: "White Beetle 1 on board", exact: true }).click();
        await expect(page.getByRole("button", { name: /^Move to board position / })).toHaveCount(0);
        await page.getByRole("button", { name: "White Ant 2 in reserve", exact: true }).click();
        await expect(page.getByRole("button", { name: /^Move to board position / })).toHaveCount(0);
      }, { box: true });

      await test.step("Play turns six and seven", async () => {
        await placePiece(userTwo, "Black Grasshopper 1", boardPositions.blackGrasshopper);
        await movePiece(page, "White Beetle 1", boardPositions.beetleMove);
      }, { box: true });

      await test.step("Verify the seven-move history", async () => {
        await reviewHistory(page, 7, [
          [3, "bQ"],
          [6, "wB1"],
        ]);
        await showTab(page, "Game");
      }, { box: true });
    }, { box: true });

    await test.step("Negotiate rejected and accepted takebacks", async () => {
      await test.step("Reject a takeback request", async () => {
        await confirmControl(userTwo, "Request Takeback");
        await expect(page.getByText("Opponent wants a takeback")).toBeVisible();
        await page.getByTitle("Reject Takeback").click();
        await reviewHistory(page, 7);
        await showTab(page, "Game");
      }, { box: true });

      await test.step("Accept a takeback request", async () => {
        await confirmControl(userTwo, "Request Takeback");
        await expect(page.getByText("Opponent wants a takeback")).toBeVisible();
        await page.getByTitle("Accept Takeback").click();
        await reviewHistory(page, 6, [[5, "bG1"]]);
      }, { box: true });
    }, { box: true });

    await test.step("Negotiate and confirm a draw", async () => {
      await test.step("Reject a draw offer", async () => {
        await showTab(page, "Game");
        await confirmControl(page, "Offer Draw");
        await expect(userTwo.getByText("Opponent offers a draw")).toBeVisible();
        await userTwo.getByTitle("Reject Draw").click();
        await expect(page.getByTitle("Offer Draw")).toBeVisible();
      }, { box: true });

      await test.step("Accept a draw offer and verify the outcome", async () => {
        await confirmControl(page, "Offer Draw");
        await expect(userTwo.getByText("Opponent offers a draw")).toBeVisible();
        await userTwo.getByTitle("Accept Draw").click();
        await showTab(page, "History");
        await showTab(userTwo, "History");
        await expect(page.getByText("Draw agreed", { exact: true })).toBeVisible();
        await expect(userTwo.getByText("Draw agreed", { exact: true })).toBeVisible();
      }, { box: true });
    }, { box: true });
  } finally {
    await userTwoContext?.close();
  }
});

test("two players can chat and resign after four turns", async ({ browser, page }, testInfo) => {
  test.setTimeout(90_000);

  const viewport = page.viewportSize();
  let userTwoContext: BrowserContext | undefined;

  try {
    const userTwo = await test.step("Sign in and start a game", async () => {
      userTwoContext = await browser.newContext({
        baseURL,
        ...(viewport ? { viewport } : {}),
      });
      const userTwo = await userTwoContext.newPage();

      await test.step("Sign in both players", async () => {
        await signIn(page, "user_1");
        await signIn(userTwo, "user_2");
      }, { box: true });

      await test.step("Create and accept a white challenge", async () => {
        await createTargetedWhiteChallenge(page, "user_2");
        const challenge = challengeFrom(userTwo, "user_1");
        await expect(challenge).toBeVisible();
        await challenge.getByRole("button", { name: "Accept Challenge" }).click();
      }, { box: true });

      await test.step("Confirm both players are ready to play", async () => {
        await expect(page).toHaveURL(/\/game\//);
        await expect(userTwo).toHaveURL(page.url());
        const isMobile = testInfo.project.name.endsWith("-mobile");
        await showControlsIfMobile(page, isMobile);
        await showControlsIfMobile(userTwo, isMobile);
      }, { box: true });
      return userTwo;
    }, { box: true });

    await test.step("Play two moves each", async () => {
      await placePiece(page, "White Ant 1", boardPositions.openingAnt);
      await placePiece(userTwo, "Black Ant 1", boardPositions.blackAnt);
      await placePiece(page, "White Queen", boardPositions.whiteQueen);
      await placePiece(userTwo, "Black Queen", boardPositions.blackQueen);
    }, { box: true });

    await test.step("Exchange chat messages", async () => {
      const firstMessage = "Hello from user_1";
      const secondMessage = "Hello from user_2";
      const userOneChatTab = page.getByText("Chat", { exact: true });
      const userTwoChatTab = userTwo.getByText("Chat", { exact: true });

      await test.step("Send a message to a player outside chat", async () => {
        await showTab(page, "Chat");
        const chatInput = page.getByLabel("Chat message");
        await chatInput.fill(firstMessage);
        await chatInput.press("Enter");
        await expect(page.getByRole("log", { name: "Chat messages" })).toContainText(firstMessage);
      }, { box: true });

      await test.step("Show a red alert for the unread message", async () => {
        await expect(userTwoChatTab).toHaveClass(/ui-button-danger/);
      }, { box: true });

      await test.step("Read the message and reply from chat", async () => {
        await showTab(userTwo, "Chat");
        await expect(userTwo.getByRole("log", { name: "Chat messages" })).toContainText(firstMessage);
        const chatInput = userTwo.getByLabel("Chat message");
        await chatInput.fill(secondMessage);
        await chatInput.press("Enter");
        await expect(userTwo.getByRole("log", { name: "Chat messages" })).toContainText(secondMessage);
      }, { box: true });

      await test.step("Receive the reply while chat is open", async () => {
        await expect(page.getByRole("log", { name: "Chat messages" })).toContainText(secondMessage);
      }, { box: true });

      await test.step("Keep the Chat tab clear while the message is read", async () => {
        await expect(userOneChatTab).not.toHaveClass(/ui-button-danger/);
      }, { box: true });
    }, { box: true });

    await test.step("Resign and confirm the outcome", async () => {
      await showTab(page, "Game");
      await confirmControl(page, "Resign");
      await expect(page.getByRole("alert")).toContainText("You resigned");
      await expect(userTwo.getByRole("alert")).toContainText("user_1 resigned");
    }, { box: true });
  } finally {
    await userTwoContext?.close();
  }
});
