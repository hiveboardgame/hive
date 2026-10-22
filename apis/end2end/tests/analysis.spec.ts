import {
  board,
  boardPiece,
  boardSnapshot,
  boardViewport,
  expectPieceAt,
  pieceSeparation,
  reservePiece,
  targetAt,
  targetPosition,
  targets,
} from "../support/game/board";
import { boardPositions, openingFile } from "../support/game/opening";
import { historyStep } from "../support/game/history";
import { test, expect } from "../support/fixtures";
import {
  currentNode,
  enableDragging,
  expandVariations,
  loadOpening,
  placeFromReserve,
  selectNode,
  uploadOpening,
} from "../support/analysis";
import { activate, center, mouseDrag, mousePan, withChromiumTouch } from "../support/gestures";

test("Analysis input, history, and branches keep board and reserve synchronized", { tag: "@interaction" }, async ({ page, hasTouch }) => {
  await loadOpening(page);
  const original = await currentNode(page);
  await historyStep(page, "Previous", hasTouch);
  await expectPieceAt(page, "White Beetle 1", boardPositions.whiteBeetle);
  const parent = await currentNode(page);

  const position = await placeFromReserve(page, "White Ant 2", hasTouch);
  const alternative = await currentNode(page);
  expect(alternative).not.toBe(original);
  await expect(reservePiece(page, "White Ant 2")).toHaveCount(0);

  await selectNode(page, original, hasTouch);
  await expectPieceAt(page, "White Beetle 1", boardPositions.whiteQueen, 1);
  await expect(boardPiece(page, "White Ant 2")).toHaveCount(0);
  await expect(reservePiece(page, "White Ant 2")).toBeVisible();

  await expandVariations(page, parent, hasTouch);
  await selectNode(page, alternative, hasTouch);
  await expectPieceAt(page, "White Ant 2", position);
  await expectPieceAt(page, "White Beetle 1", boardPositions.whiteBeetle);
  await selectNode(page, parent, hasTouch);
  await expect(boardPiece(page, "White Ant 2")).toHaveCount(0);
  await expect(reservePiece(page, "White Ant 2")).toBeVisible();
});

test("Mouse drags cross reserve SVGs and land on a raised stack", { tag: ["@interaction", "@mouse"] }, async ({ data, player, isMobileLayout }) => {
  const { users } = await data.users(1);
  const { page } = await player(users[0]);
  await enableDragging(page, users[0].username);
  await page.getByRole("button", { name: isMobileLayout ? "Open navigation menu" : "Learn", exact: true }).click();
  await page.getByRole("link", { name: "Analysis", exact: true }).click();
  await uploadOpening(page);

  const reserve = reservePiece(page, "Black Beetle 1");
  const spawnPosition = "14, 15";
  const spawnTarget = targetAt(page, spawnPosition);
  // Targets are created only after the drag crosses its pointer threshold.
  await mouseDrag(page, reserve, spawnTarget);
  await expectPieceAt(page, "Black Beetle 1", spawnPosition);
  await expect(reserve).toHaveCount(0);

  await mouseDrag(page, boardPiece(page, "White Beetle 1"), targetAt(page, boardPositions.openingAnt));
  await expectPieceAt(page, "White Beetle 1", boardPositions.openingAnt, 1);
  await expectPieceAt(page, "White Queen", boardPositions.whiteQueen);
});

test("Right press inspects a stack and annotation mode owns the same gesture", { tag: ["@interaction", "@mouse"] }, async ({ page }) => {
  await loadOpening(page);
  const beetle = boardPiece(page, "White Beetle 1");
  let point = await center(beetle);
  const queen = boardPiece(page, "White Queen");
  const collapsedSeparation = await pieceSeparation(beetle, queen);
  await page.mouse.move(point.x, point.y);
  await page.mouse.down({ button: "right" });
  try {
    await expect.poll(() => pieceSeparation(beetle, queen)).toBeGreaterThan(collapsedSeparation);
  } finally {
    await page.mouse.up({ button: "right" });
  }
  await expect(beetle).toHaveAttribute("data-stack-level", "1");

  const annotate = page.getByTitle("Annotate (markers, arrows, highlights)", { exact: true });
  await annotate.click();
  point = await center(beetle);
  await page.mouse.move(point.x, point.y);
  await page.mouse.down({ button: "right" });
  try {
    await expect(board(page).locator('g[pointer-events="none"] > *')).toHaveCount(1);
    await expect(beetle).toHaveAttribute("data-stack-level", "1");
  } finally {
    await page.mouse.up({ button: "right" });
  }
  // The preview becomes a persistent mark on release.
  await expect(board(page).locator('g[pointer-events="none"] > *')).toHaveCount(1);
  await annotate.click();
  // Leaving the board clears an unfinished preview; the committed mark must survive.
  await expect(board(page).locator('g[pointer-events="none"] > *')).toHaveCount(1);
  const position = await placeFromReserve(page, "Black Ant 2", false);
  await expectPieceAt(page, "Black Ant 2", position);
});

test("Panning and wheel zoom preserve board hit testing", { tag: ["@interaction", "@mouse"] }, async ({ page }) => {
  const time = new Date();
  await page.clock.install({ time });
  await page.clock.setFixedTime(time);
  await loadOpening(page);
  const svg = board(page);
  // Reproduce a release before the next frame without relying on machine speed.
  await page.clock.pauseAt(time);
  await page.clock.runFor(32);
  const beforePan = await boardViewport(page);
  await mousePan(page, svg);
  await page.clock.runFor(32);
  await page.clock.resume();
  await expect.poll(async () => (await boardViewport(page)).origin).not.toEqual(beforePan.origin);
  const beforeZoom = await boardViewport(page);
  const point = await center(boardPiece(page, "White Beetle 1"));
  await page.mouse.move(point.x, point.y);
  await page.mouse.wheel(0, -180);
  await expect.poll(async () => (await boardViewport(page)).size).not.toEqual(beforeZoom.size);

  await activate(boardPiece(page, "Black Grasshopper 1"), false);
  const target = targets(page).first();
  await expect(target).toBeVisible();
  const position = await targetPosition(target);
  await target.click();
  await expectPieceAt(page, "Black Grasshopper 1", position);
});

test("Native touch long press and pinch leave the board usable", { tag: ["@interaction", "@chromium-touch"] }, async ({ page }) => {
  await loadOpening(page);
  const beetle = boardPiece(page, "White Beetle 1");
  const point = await center(beetle);
  const queen = boardPiece(page, "White Queen");
  const collapsedSeparation = await pieceSeparation(beetle, queen);
  await withChromiumTouch(page, async touch => {
    await touch.start([point]);
    // Poll the actual expansion while holding the finger; no fixed sleep substitutes for readiness.
    await expect.poll(() => pieceSeparation(beetle, queen)).toBeGreaterThan(collapsedSeparation);
    await touch.end();
    await expect(beetle).toHaveAttribute("data-stack-level", "1");

    const beforePinch = await boardViewport(page);
    const middle = await center(beetle);
    await touch.start([{ x: middle.x - 25, y: middle.y }, { x: middle.x + 25, y: middle.y }]);
    await touch.move([{ x: middle.x - 40, y: middle.y }, { x: middle.x + 40, y: middle.y }]);
    await expect.poll(async () => (await boardViewport(page)).size).not.toEqual(beforePinch.size);
    await touch.end();
  });

  await placeFromReserve(page, "Black Ant 2", true);
});

test("Saving and loading an analysis restores the selected branch", { tag: "@interaction" }, async ({ page, hasTouch }, testInfo) => {
  await loadOpening(page);
  await historyStep(page, "Previous", hasTouch);
  await placeFromReserve(page, "White Ant 2", hasTouch);
  const expectedBoard = await boardSnapshot(page);
  const expectedNode = await currentNode(page);

  const downloading = page.waitForEvent("download");
  await activate(page.getByRole("button", { name: "Save", exact: true }), hasTouch);
  const file = testInfo.outputPath("analysis.json");
  await (await downloading).saveAs(file);

  // Replace the current document before loading the saved file, including reused node IDs.
  await page.locator("#load-analysis").setInputFiles(openingFile);
  await expectPieceAt(page, "White Beetle 1", boardPositions.whiteQueen, 1);
  await expect(boardPiece(page, "White Ant 2")).toHaveCount(0);
  await page.locator("#load-analysis").setInputFiles(file);
  await expect.poll(() => boardSnapshot(page)).toEqual(expectedBoard);
  expect(await currentNode(page)).toBe(expectedNode);
  await historyStep(page, "Previous", hasTouch);
  await expect(boardPiece(page, "White Ant 2")).toHaveCount(0);
  await expect(reservePiece(page, "White Ant 2")).toBeVisible();
  await expandVariations(page, await currentNode(page), hasTouch);
  await selectNode(page, expectedNode, hasTouch);
  await expect.poll(() => boardSnapshot(page)).toEqual(expectedBoard);
});
