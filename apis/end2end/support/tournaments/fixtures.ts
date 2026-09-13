import { test as base, expect, type BrowserContext } from "@playwright/test";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { signIn } from "../browser/authentication";
import { attachFailureDiagnostics, logBrowserDiagnostics } from "../browser/diagnostics";
import type { Player } from "../browser/player";
import { TournamentData, type Account } from "./data";

type TournamentPlayer = Player & Account;
type TournamentFixture = {
  data: TournamentData;
  organizer: TournamentPlayer;
  member: TournamentPlayer;
  login(account: Account): Promise<TournamentPlayer>;
  another(admin?: boolean): Promise<TournamentPlayer>;
};

export const test = base.extend<{
  tournament: TournamentFixture;
  tournamentAdmin: TournamentPlayer;
  tournamentOutsider: TournamentPlayer;
}>({
  // Additional role logins are setup, and must not consume the scenario's 60s budget.
  tournamentAdmin: [async ({ tournament }, use) => {
    await use(await tournament.another(true));
  }, { timeout: 90_000 }],
  tournamentOutsider: [async ({ tournament }, use) => {
    await use(await tournament.another());
  }, { timeout: 90_000 }],
  tournament: [async ({ browser, baseURL }, use, testInfo) => {
    if (!baseURL || !process.env.PLAYWRIGHT_DATABASE_URL) throw new Error("Tournament UI tests require PLAYWRIGHT_DATABASE_URL and a baseURL.");
    // Outside Playwright's cleaned outputDir, so another run cannot erase crash recovery.
    const manifest = path.resolve(__dirname, "../../test-results/tournament-ownership", `${randomUUID()}.json`);
    const data = new TournamentData(process.env.PLAYWRIGHT_DATABASE_URL, manifest);
    const contexts: BrowserContext[] = [];
    const players: Player[] = [];
    const close = async () => {
      const outcomes = await Promise.allSettled(contexts.map(async context => {
        try { await context.unrouteAll({ behavior: "ignoreErrors" }); } finally { await context.close(); }
      }));
      const failed = outcomes.find(o => o.status === "rejected");
      if (failed?.status === "rejected") throw failed.reason;
    };
    data.onLost = () => { void close().catch(() => {}); };
    let failed = false;
    const login = async (account: Account) => {
      data.assertHeld();
      const context = await browser.newContext();
      contexts.push(context);
      const page = await context.newPage();
      const player = { ...account, context, page };
      players.push(player);
      logBrowserDiagnostics(page, account.username);
      await test.step(`Sign in as ${account.admin ? "admin" : "player"} ${account.username}`,
        () => signIn(page, account.username, baseURL), { box: true });
      data.assertHeld();
      return player;
    };
    try {
      await data.open();
      const [owner, participant] = await data.accounts(2);
      const organizer = await login(owner);
      const member = await login(participant);
      await use({ data, organizer, member, login,
        another: async (admin = false) => login((await data.accounts(1, admin))[0]) });
      data.assertHeld();
    } catch (error) {
      failed = true;
      throw error;
    } finally {
      if (failed || testInfo.status !== testInfo.expectedStatus) await attachFailureDiagnostics(players, testInfo);
      let cleanupError: unknown;
      for (const dispose of [close, () => test.step("Remove this attempt's tournament fixtures", () => data.cleanup(),
        { box: true, timeout: 30_000 }), () => data.close()]) {
        try { await dispose(); } catch (error) { cleanupError ??= error; }
      }
      if (cleanupError) {
        await testInfo.attach("Tournament ownership manifest for recovery", { path: manifest, contentType: "application/json" }).catch(() => {});
        if (failed || testInfo.status !== "passed") await testInfo.attach("Tournament cleanup failure", {
          body: String(cleanupError), contentType: "text/plain",
        });
        else throw cleanupError;
      }
    }
  }, { timeout: 270_000 }],
});
export { expect };
