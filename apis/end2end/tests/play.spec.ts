import type { WebSocketRoute } from "@playwright/test";
import { expect, test } from "../support/fixtures";
import { signIn } from "../support/browser/authentication";
import { reloadHydrated } from "../support/browser/hydration";
import { expectPieceAt, placePiece } from "../support/game/board";
import { confirmControl, showControlsIfMobile } from "../support/game/controls";
import { boardPositions, playOpening } from "../support/game/opening";
import { startGame } from "../support/game/setup";
import { showTab } from "../support/game/panels";
import { historyControl } from "../support/game/history";

// Two independent clients and deliberate reloads include fresh browser hydration.
test.describe.configure({ timeout: 90_000 });

test("an accepted challenge opens a live game whose moves survive reload @journey", async ({ data, player, isMobileLayout }) => {
  const { users } = await data.users(2);
  const [white, black] = await Promise.all(users.map(account => player(account)));
  await startGame({ white, black, isMobileLayout });
  await playOpening({ userOne: white, userTwo: black }, 2);

  await placePiece(white.page, "White Queen", boardPositions.whiteQueen);
  await expectPieceAt(black.page, "White Queen", boardPositions.whiteQueen);
  await reloadHydrated(black.page);
  await expectPieceAt(black.page, "White Queen", boardPositions.whiteQueen);
  await placePiece(black.page, "Black Queen", boardPositions.blackQueen);
  await expectPieceAt(white.page, "Black Queen", boardPositions.blackQueen);
});

test("a disconnected player receives the missed move and can continue @journey", async ({ data, player }) => {
  const { users, gameId } = await data.game(2);
  const gamePath = `/game/${gameId}`;
  const [white, black] = await Promise.all(users.map(account => player(account, { authenticate: false })));
  let disconnected = false;
  let transport: { client: WebSocketRoute; server: WebSocketRoute } | undefined;
  // Forward the real protocol unchanged; only sever the transport during the fault.
  await black.page.routeWebSocket(/\/ws\//, client => {
    if (disconnected) {
      void client.close({ code: 1013, reason: "Test network interruption" });
      return;
    }
    transport = { client, server: client.connectToServer() };
  });
  await Promise.all([signIn(white.page, users[0], gamePath), signIn(black.page, users[1], gamePath)]);
  await expectPieceAt(black.page, "Black Ant 1", boardPositions.blackAnt);
  await expect.poll(() => Boolean(transport)).toBe(true);
  await expect(black.page.getByText("Connecting..", { exact: true })).toHaveCount(0);

  disconnected = true;
  await black.context.setOffline(true);
  await Promise.all([
    transport!.client.close({ code: 1012, reason: "Test network interruption" }),
    transport!.server.close(),
  ]);
  try {
    await expect(black.page.getByText("Connecting..", { exact: true })).toBeVisible();
    await placePiece(white.page, "White Queen", boardPositions.whiteQueen);
    await expectPieceAt(white.page, "White Queen", boardPositions.whiteQueen);
  } finally {
    disconnected = false;
    await black.context.setOffline(false);
  }

  await expectPieceAt(black.page, "White Queen", boardPositions.whiteQueen);
  await placePiece(black.page, "Black Queen", boardPositions.blackQueen);
  await expectPieceAt(white.page, "Black Queen", boardPositions.blackQueen);
});

test("history stays local and an accepted takeback updates both boards @journey", async ({ data, player, isMobileLayout: mobile }) => {
  const { users, gameId } = await data.game(7);
  const [white, black] = await Promise.all(users.map(account => player(account, { path: `/game/${gameId}` })));
  await expectPieceAt(white.page, "White Beetle 1", boardPositions.beetleMove, 1);
  await showControlsIfMobile(white.page, mobile);
  if (!mobile) await showTab(white.page, "History", false);
  await historyControl(white.page, "Previous").click();
  await expectPieceAt(white.page, "White Beetle 1", boardPositions.whiteBeetle);
  await expectPieceAt(black.page, "White Beetle 1", boardPositions.beetleMove, 1);
  await historyControl(white.page, "Last").click();
  await expectPieceAt(white.page, "White Beetle 1", boardPositions.beetleMove, 1);

  if (!mobile) await showTab(white.page, "Game", false);
  await showControlsIfMobile(black.page, mobile);
  await confirmControl(black.page, "Request Takeback");
  await white.page.getByTitle("Accept Takeback", { exact: true }).click();
  for (const page of [white.page, black.page]) {
    await expectPieceAt(page, "White Beetle 1", boardPositions.whiteBeetle);
    await expectPieceAt(page, "Black Grasshopper 1", boardPositions.blackGrasshopper);
  }
  await reloadHydrated(black.page);
  await expectPieceAt(black.page, "White Beetle 1", boardPositions.whiteBeetle);
});

test("game chat reaches the other player and survives reload @journey", async ({ data, player, isMobileLayout: mobile }) => {
  const { users, gameId } = await data.game(2);
  const [white, black] = await Promise.all(users.map(account => player(account, { path: `/game/${gameId}` })));
  for (const page of [white.page, black.page]) await showTab(page, "Chat", mobile);
  const message = `A game message from ${white.username}`;
  await white.page.getByLabel("Chat message", { exact: true }).fill(message);
  await white.page.getByLabel("Chat message", { exact: true }).press("Enter");
  await expect(black.page.getByRole("log", { name: "Chat messages" })).toContainText(message);
  await reloadHydrated(black.page);
  await showTab(black.page, "Chat", mobile);
  await expect(black.page.getByRole("log", { name: "Chat messages" })).toContainText(message);
});
