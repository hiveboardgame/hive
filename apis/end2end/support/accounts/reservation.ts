import type { Client } from "pg";
import type { TestAccount } from "./catalog";

export type AccountState = {
  challenges: { nanoid: string; challenger_id: string; opponent_id: string | null }[];
  games: {
    nanoid: string; white_id: string; black_id: string; turn: number;
    tournament_id: string | null;
  }[];
};

export class AccountReservation {
  private closing = false;
  private failure?: Error;
  private listeners = new Set<(error: Error) => void>();
  private readonly heartbeat: ReturnType<typeof setInterval>;
  private checking = false;

  constructor(
    private client: Client,
    readonly accounts: readonly TestAccount[],
    readonly pool: readonly TestAccount[],
  ) {
    client.on("error", this.lose);
    client.on("end", this.lose);
    this.heartbeat = setInterval(() => {
      if (this.checking || this.closing || this.failure) return;
      this.checking = true;
      void client.query("SELECT 1").catch(this.lose).finally(() => {
        this.checking = false;
      });
    }, 5_000);
    this.heartbeat.unref();
  }

  private lose = () => {
    if (this.closing || this.failure) return;
    this.failure = new Error(`Lost the database connection holding the test account reservation for ${this.accounts.map(account => account.username).join(" / ")}.`);
    clearInterval(this.heartbeat);
    for (const listener of this.listeners) listener(this.failure);
  };

  assertHeld() {
    if (this.failure) throw this.failure;
    if (this.closing) throw new Error("The test account reservation has already been released.");
  }

  onLost(listener: (error: Error) => void) {
    this.listeners.add(listener);
    if (this.failure) listener(this.failure);
    return () => { this.listeners.delete(listener); };
  }

  async state(): Promise<AccountState> {
    this.assertHeld();
    const ids = this.accounts.map(account => account.id);
    try {
      const challenges = await this.client.query<AccountState["challenges"][number]>(
        `SELECT nanoid, challenger_id, opponent_id FROM challenges
         WHERE challenger_id = ANY($1::uuid[]) OR opponent_id = ANY($1::uuid[])`,
        [ids],
      );
      const games = await this.client.query<AccountState["games"][number]>(
        `SELECT nanoid, white_id, black_id, turn, tournament_id FROM games
         WHERE NOT finished AND (white_id = ANY($1::uuid[]) OR black_id = ANY($1::uuid[]))`,
        [ids],
      );
      this.assertHeld();
      return { challenges: challenges.rows, games: games.rows };
    } catch {
      this.assertHeld();
      throw new Error("Unable to inspect server state for the reserved test accounts.");
    }
  }

  async release() {
    if (this.closing) return;
    this.closing = true;
    clearInterval(this.heartbeat);
    // Closing this dedicated session releases its advisory lock, also on errors.
    await this.client.end();
    this.listeners.clear();
  }
}

