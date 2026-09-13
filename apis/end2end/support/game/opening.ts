import { test } from "@playwright/test";
import type { Players } from "../browser/session";
import { expectPieceAt, movePiece, placePiece } from "./board";

export const boardPositions = {
  openingAnt: "16, 16",
  blackAnt: "15, 16",
  whiteQueen: "16, 17",
  blackQueen: "15, 15",
  whiteBeetle: "17, 16",
  blackGrasshopper: "16, 14",
  beetleMove: "16, 17",
} as const;

// Only the history and takeback cases need the full stacked-beetle opening.
export async function playOpening({ userOne, userTwo }: Players, count: 2 | 5 | 7) {
  const moves = [
    [userOne, "White Ant 1", boardPositions.openingAnt],
    [userTwo, "Black Ant 1", boardPositions.blackAnt],
    [userOne, "White Queen", boardPositions.whiteQueen],
    [userTwo, "Black Queen", boardPositions.blackQueen],
    [userOne, "White Beetle 1", boardPositions.whiteBeetle],
    [userTwo, "Black Grasshopper 1", boardPositions.blackGrasshopper],
    [userOne, "White Beetle 1", boardPositions.beetleMove],
  ] as const;
  for (const [index, [player, piece, position]] of moves.slice(0, count).entries()) {
    await test.step(`Turn ${index + 1}: ${piece} to ${position}`, async () => {
      const action = index === 6 ? movePiece : placePiece;
      await action(player.page, piece, position);
      for (const observer of [userOne, userTwo]) {
        await expectPieceAt(observer.page, piece, position, index === 6 ? 1 : 0);
      }
    }, { box: true });
  }
}
