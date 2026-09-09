import { Client } from "pg";
import { setTimeout as delay } from "node:timers/promises";
import { usernames, publicTimeControls, type TestAccount } from "./catalog";
import { AccountReservation } from "./reservation";

// All runs against a database must use this namespace and per-account lock keys.
// Session locks never lock application rows or hold a transaction open.
const lockNamespace = 0x48495645; // HIVE
export const accountWaitTimeout = 180_000;

export async function reserveAccounts(count: 1 | 2, {
  connectionString = process.env.PLAYWRIGHT_DATABASE_URL,
  timeout = accountWaitTimeout,
}: { connectionString?: string; timeout?: number } = {}): Promise<AccountReservation> {
  if (!connectionString) {
    throw new Error("Set PLAYWRIGHT_DATABASE_URL to the test database served by PLAYWRIGHT_BASE_URL.");
  }
  let client: Client;
  try {
    client = new Client({
      connectionString,
      connectionTimeoutMillis: 5_000,
      query_timeout: 5_000,
      keepAlive: true,
      application_name: `hive-playwright-${process.pid}`,
    });
  } catch {
    throw new Error("Cannot connect to PLAYWRIGHT_DATABASE_URL; check test database access.");
  }
  // Do not let pg's asynchronous error event escape or print connection details.
  let disconnected = false;
  const disconnect = () => { disconnected = true; };
  client.on("error", disconnect);
  client.on("end", disconnect);
  let transferred = false;
  try {
    try {
      await client.connect();
    } catch {
      throw new Error("Cannot connect to PLAYWRIGHT_DATABASE_URL; check test database access.");
    }
    let accounts: TestAccount[];
    try {
      const result = await client.query<TestAccount>(
        `SELECT id, username FROM users
         WHERE username = ANY($1::text[]) AND normalized_username = username
           AND email = username || '@example.test' AND email_verified AND NOT admin
           AND NOT deleted AND NOT bot
         ORDER BY username`,
        [usernames],
      );
      accounts = result.rows;
    } catch {
      throw new Error("Cannot validate test accounts; apply application and testware migrations first.");
    }
    if (accounts.length !== usernames.length || usernames.some((name, i) => accounts[i].username !== name)) {
      throw new Error("The account pool requires verified, non-admin user_1 through user_8; apply testware migrations.");
    }
    const deadline = Date.now() + timeout;
    let backoff = 100;
    do {
      const selected: TestAccount[] = [];
      try {
        for (let pool = 0; pool < count; pool++) {
          const first = Math.floor(Math.random() * publicTimeControls.length);
          for (let offset = 0; offset < publicTimeControls.length; offset++) {
            if (disconnected) throw new Error();
            const index = ((first + offset) % publicTimeControls.length) * 2 + pool;
            const result = await client.query<{ acquired: boolean }>(
              "SELECT pg_try_advisory_lock($1::int, $2::int) AS acquired",
              [lockNamespace, index + 1],
            );
            if (result.rows[0].acquired) {
              selected.push(accounts[index]);
              break;
            }
          }
          if (selected.length !== pool + 1) break;
        }
        if (disconnected) throw new Error();
        if (selected.length === count) {
          const reservation = new AccountReservation(client, selected, accounts);
          client.off("error", disconnect);
          client.off("end", disconnect);
          transferred = true;
          return reservation;
        }
        // Never hold a primary account while waiting for a partner. This client
        // is dedicated to this reservation, so all its locks belong to us.
        await client.query("SELECT pg_advisory_unlock_all()");
      } catch {
        throw new Error("Database connection failed while reserving test accounts.");
      }
      const remaining = deadline - Date.now();
      if (remaining <= 0) break;
      await delay(Math.min(backoff, remaining));
      backoff = Math.min(backoff * 2, 1_000);
    } while (Date.now() < deadline);
    throw new Error(`Unable to reserve ${count} test account(s) within ${timeout}ms; the required pool is occupied.`);
  } finally {
    if (!transferred) await client.end().catch(() => {});
  }
}
