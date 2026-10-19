import { test } from "@playwright/test";
import type { Players } from "../browser/player";
import { expectPieceAt, movePiece, placePiece } from "./board";
import opening from "../../assets/stacked-opening.json";

export const boardPositions = Object.fromEntries(opening.map(move => [move.key, `${move.position.q}, ${move.position.r}`]));
export const openingFile = {
  name: "stacked-opening.pgn",
  mimeType: "text/plain",
  buffer: Buffer.from('[GameType "Base"]\n\n' + opening.map((move, index) => `${index + 1}. ${move.notation}`).join("\n")),
};

export async function playOpening({ userOne, userTwo }: Players, count: 2 | 5 | 7) {
  for (const [index, move] of opening.slice(0, count).entries()) {
    const position = boardPositions[move.key];
    const player = index % 2 === 0 ? userOne : userTwo;
    await test.step(`Turn ${index + 1}: ${move.name} to ${position}`, async () => {
      const action = move.level > 0 ? movePiece : placePiece;
      await action(player.page, move.name, position);
      for (const observer of [userOne, userTwo]) {
        await expectPieceAt(observer.page, move.name, position, move.level);
      }
    }, { box: true });
  }
}
