import { expect, test } from "@playwright/test";
import { cleanUpAccounts } from "../support/accounts/cleanup";
import type { AccountReservation, AccountState } from "../support/accounts/reservation";
import type { Player } from "../support/browser/player";

const accounts = [{ id: "one", username: "user_1" }, { id: "two", username: "user_2" }] as const;

function setup(state: AccountState, single = false) {
  const actions: string[] = [];
  const reservation = {
    accounts: single ? accounts.slice(0, 1) : accounts,
    pool: [...accounts, { id: "three", username: "user_3" }],
    state: async () => state,
    assertHeld: () => {},
  } as unknown as AccountReservation;
  const players = reservation.accounts.map(account => {
    let url = "";
    let clicks = 0;
    return {
      ...account,
      page: {
        goto: async (value: string) => { url = value; clicks = 0; actions.push(`${account.id}:${url}`); },
        getByRole: () => ({
          getByRole: (_role: string, { name }: { name: string }) => ({
            first: () => ({
              click: async () => {
                actions.push(`${account.id}:${name}`);
                const matches = (row: AccountState["challenges"][number]) => name === "Cancel Challenge"
                  ? row.challenger_id === account.id : row.opponent_id === account.id;
                // A single row can group multiple IDs.
                state.challenges = state.challenges.filter(row => !matches(row));
              },
            }),
          }),
        }),
        getByTitle: (title: string) => ({
          click: async () => {
            actions.push(`${account.id}:${title}`);
            if ((title === "Abort" || title === "Resign") && ++clicks === 2) {
              state.games = state.games.filter(row => !url.endsWith(row.nanoid));
            }
          },
        }),
      },
    };
  }) as unknown as Player[];
  return { reservation, players, actions };
}

test("cleanup drains grouped challenges and aborts or resigns through a reserved participant", async () => {
  const { reservation, players, actions } = setup({
    challenges: [
      { nanoid: "public", challenger_id: "one", opponent_id: null },
      { nanoid: "public2", challenger_id: "one", opponent_id: null },
      { nanoid: "direct", challenger_id: "two", opponent_id: "one" },
    ],
    games: [
      { nanoid: "opening", white_id: "one", black_id: "two", turn: 1, tournament_id: null },
      { nanoid: "started", white_id: "two", black_id: "one", turn: 2, tournament_id: null },
    ],
  });
  await cleanUpAccounts(reservation, players, true);
  expect(actions).toEqual([
    "one:/", "one:Cancel Challenge",
    "one:/", "one:Decline Challenge",
    "one:/game/opening", "one:Show controls", "one:Abort", "one:Abort",
    "one:/game/started", "one:Show controls", "one:Resign", "one:Resign",
  ]);
  await cleanUpAccounts(reservation, players, true);
  expect(actions).toHaveLength(12);
});

test("a single user recovers incoming challenges and games against unreserved seeded users", async () => {
  const { reservation, players, actions } = setup({
    challenges: [{ nanoid: "incoming", challenger_id: "three", opponent_id: "one" }],
    games: [{ nanoid: "started", white_id: "three", black_id: "one", turn: 3, tournament_id: null }],
  }, true);
  await cleanUpAccounts(reservation, players, false);
  expect(actions).toEqual([
    "one:/", "one:Decline Challenge", "one:/game/started", "one:Resign", "one:Resign",
  ]);
});

test("cleanup validates all state before mutating, rejecting external users and tournaments", async () => {
  for (const state of [
    { challenges: [{ nanoid: "foreign", challenger_id: "other", opponent_id: "one" }], games: [] },
    { challenges: [{ nanoid: "foreign", challenger_id: "one", opponent_id: "other" }], games: [] },
    {
      challenges: [{ nanoid: "own", challenger_id: "one", opponent_id: null }],
      games: [{ nanoid: "foreign", white_id: "one", black_id: "other", turn: 2, tournament_id: null }],
    },
    { challenges: [], games: [{ nanoid: "tournament", white_id: "one", black_id: "two", turn: 3, tournament_id: "t" }] },
  ]) {
    const { reservation, players, actions } = setup(state);
    await expect(cleanUpAccounts(reservation, players, false)).rejects.toThrow("cleanup will not modify it");
    expect(actions).toEqual([]);
  }
});

for (const duringClick of [false, true]) {
  test(`cleanup tolerates a game resolved by its opponent ${duringClick ? "during a click" : "during navigation"}`, async () => {
    const state = { challenges: [], games: [{ nanoid: "game", white_id: "one", black_id: "two", turn: 3, tournament_id: null }] };
    const { reservation, players } = setup(state, true);
    if (duringClick) {
      players[0].page.getByTitle = (() => ({ click: async () => {
        state.games = [];
        throw new Error("game already ended");
      } })) as unknown as Player["page"]["getByTitle"];
    } else {
      players[0].page.goto = async () => { state.games = []; return null; };
    }
    await cleanUpAccounts(reservation, players, false);
    expect(state.games).toEqual([]);
  });
}

test("a click failure is not swallowed while the game is still outstanding", async () => {
  const { reservation, players } = setup({
    challenges: [], games: [{ nanoid: "game", white_id: "one", black_id: "two", turn: 3, tournament_id: null }],
  });
  players[0].page.getByTitle = (() => ({ click: async () => { throw new Error("click failed"); } })) as unknown as Player["page"]["getByTitle"];
  await expect(cleanUpAccounts(reservation, players, false)).rejects.toThrow("click failed");
});

test("cleanup stops before browser actions when the reservation is lost", async () => {
  const { reservation, players, actions } = setup({
    challenges: [{ nanoid: "own", challenger_id: "one", opponent_id: null }], games: [],
  });
  reservation.assertHeld = () => { throw new Error("lost reservation"); };
  await expect(cleanUpAccounts(reservation, players, false)).rejects.toThrow("lost reservation");
  expect(actions).toEqual([]);
});
