import { test, type Browser, type BrowserContext, type TestInfo } from "@playwright/test";
import { reserveAccounts } from "../accounts/pool";
import { cleanUpAccounts } from "../accounts/cleanup";
import { signIn } from "./authentication";
import type { Player } from "./player";

export type Players = Readonly<{ userOne: Player; userTwo: Player; publicTimeControl: string }>;

// The injected actions let lifecycle checks exercise failures without a server.
export async function withUsers({
  browser, count, baseURL, isMobileLayout, testInfo,
  reserve = reserveAccounts, authenticate = signIn, cleanup = cleanUpAccounts,
}: {
  browser: Browser;
  count: 1 | 2;
  baseURL: string | undefined;
  reserve?: typeof reserveAccounts;
  isMobileLayout: boolean;
  testInfo: Pick<TestInfo, "status" | "attach">;
  authenticate?: typeof signIn;
  cleanup?: typeof cleanUpAccounts;
}, use: (players: readonly Player[]) => Promise<void>) {
  if (!baseURL) throw new Error("Configure a Playwright baseURL before authenticating users.");
  const reservation = await reserve(count);
  const contexts = new Map<BrowserContext, Promise<void> | undefined>();
  const players: Player[] = [];
  let primaryError: unknown;
  let lostReservation: Error | undefined;
  const closeContexts = async () => {
    const results = await Promise.allSettled([...contexts].map(([context, closing]) => {
      closing ??= context.close();
      contexts.set(context, closing);
      return closing;
    }));
    const failure = results.find(result => result.status === "rejected");
    if (failure?.status === "rejected") throw failure.reason;
  };
  const unsubscribe = reservation.onLost(error => {
    lostReservation = error;
    // Interrupt browser actions immediately if we no longer own the users.
    void closeContexts().catch(() => {});
  });
  try {
    for (const account of reservation.accounts) {
      reservation.assertHeld();
      // Playwright applies the project's context options to newContext().
      const context = await browser.newContext();
      contexts.set(context, undefined);
      reservation.assertHeld();
      const page = await context.newPage();
      await test.step(`Sign in as ${account.username}`, () => authenticate(page, account.username, baseURL), { box: true });
      players.push({ ...account, context, page });
    }
    await test.step("Recover the reserved accounts", async () => {
      await cleanup(reservation, players, isMobileLayout);
    }, { box: true, timeout: 30_000 });
    reservation.assertHeld();
    await use(players);
    reservation.assertHeld();
  } catch (error) {
    primaryError = lostReservation ?? error;
    throw primaryError;
  } finally {
    let teardownError: unknown;
    for (const dispose of [
      async () => {
        if (players.length === count && !lostReservation) {
          await test.step("Clean up the reserved accounts", () => cleanup(reservation, players, isMobileLayout),
            { box: true, timeout: 30_000 });
        }
      },
      closeContexts,
      () => reservation.release(),
    ]) {
      try {
        await dispose();
      } catch (error) {
        teardownError ??= error;
      }
    }
    unsubscribe();
    if (lostReservation && lostReservation !== primaryError) teardownError = lostReservation;
    if (teardownError) {
      if (primaryError || testInfo.status !== "passed") {
        await testInfo.attach("Account cleanup failure", {
          body: String(teardownError), contentType: "text/plain",
        });
      } else {
        throw teardownError;
      }
    }
  }
}
