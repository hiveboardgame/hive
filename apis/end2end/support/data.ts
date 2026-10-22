import { execFile } from "node:child_process";
import { randomUUID } from "node:crypto";
import path from "node:path";

export type Account = Readonly<{ id: string; username: string; email: string; password: string }>;
export type Registration = Omit<Account, "id">;
export type GameData = { users: Account[]; gameId: string };
export type ArenaData = { users: Account[]; tournamentId: string };
type FixtureResult = { users: Account[]; gameId?: string; tournamentId?: string };
const root = path.resolve(__dirname, "../../..");

export class TestData {
  registration(): Registration {
    const username = `e2e_${randomUUID().replaceAll("-", "").slice(0, 12)}`;
    return { username, email: `${username}@example.test`, password: "Hive-e2e-password1!" };
  }

  users(count = 2): Promise<{ users: Account[] }> {
    return this.prepare("users", { count });
  }

  async game(plies = 7): Promise<GameData> {
    const data = await this.prepare("game", { plies });
    if (!data.gameId) throw new Error("The game fixture did not return its public identity.");
    return { users: data.users, gameId: data.gameId };
  }

  async arena(): Promise<ArenaData> {
    const data = await this.prepare("arena");
    if (!data.tournamentId) throw new Error("The arena fixture did not return its public identity.");
    return { users: data.users, tournamentId: data.tournamentId };
  }

  private prepare(scenario: string, options: { count?: number; plies?: number } = {}): Promise<FixtureResult> {
    if (!process.env.PLAYWRIGHT_DATABASE_URL) {
      throw new Error("Use npm test to start the isolated browser-test database and application.");
    }
    const target = process.env.CARGO_TARGET_DIR ?? path.join(root, ".cargo/target");
    const executable = process.env.PLAYWRIGHT_FIXTURE_BIN ?? path.join(target, "debug/hive-e2e");
    return new Promise((resolve, reject) => {
      const child = execFile(executable, [], { timeout: 60_000 }, (error, stdout, stderr) => {
        if (error) {
          reject(new Error(`Fixture creation failed: ${stderr.trim() || error.message}`));
          return;
        }
        try { resolve(JSON.parse(stdout) as FixtureResult); } catch (error) { reject(error); }
      });
      child.stdin?.end(JSON.stringify({ scenario, prefix: "e2e", ...options }));
    });
  }
}
