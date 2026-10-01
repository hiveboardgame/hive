import { Client } from "pg";
import { test, expect } from "./database";
import { reserveAccounts } from "../support/accounts/pool";
import { poolCapacity } from "../support/accounts/catalog";
import type { AccountReservation } from "../support/accounts/reservation";

test.describe("Account reservations", () => {
  for (const count of [1, 2] as const) {
    test(`${count}-user reservations are exclusive`, async ({ databaseUrl }) => {
      const reservations: AccountReservation[] = [];
      try {
        for (let i = 0; i < poolCapacity; i++) reservations.push(await reserveAccounts(count, { connectionString: databaseUrl }));
        expect(new Set(reservations.flatMap(item => item.accounts.map(account => account.username))).size).toBe(poolCapacity * count);
      } finally {
        await Promise.all(reservations.map(item => item.release()));
      }
    });

    test(`${count}-user reservation fails when its pool stays full`, async ({ databaseUrl }) => {
      const reservations: AccountReservation[] = [];
      try {
        for (let i = 0; i < poolCapacity; i++) reservations.push(await reserveAccounts(count, { connectionString: databaseUrl }));
        await expect(reserveAccounts(count, { connectionString: databaseUrl, timeout: 200 })).rejects.toThrow("required pool is occupied");
      } finally {
        await Promise.all(reservations.map(item => item.release()));
      }
    });

    test(`${count}-user waiter acquires the accounts released by their owner`, async ({ databaseUrl }) => {
      const reservations: AccountReservation[] = [];
      const observer = new Client({ connectionString: databaseUrl });
      await observer.connect();
      let waiting: Promise<{ value: AccountReservation } | { error: unknown }> | undefined;
      try {
        for (let i = 0; i < poolCapacity; i++) reservations.push(await reserveAccounts(count, { connectionString: databaseUrl }));
        const application = `hive-playwright-${process.pid}`;
        const existing = (await observer.query<{ pid: number }>(
          "SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND application_name = $1", [application],
        )).rows.map(row => row.pid);
        let settled = false;
        waiting = reserveAccounts(count, { connectionString: databaseUrl, timeout: 5_000 }).then(
          value => { settled = true; reservations.push(value); return { value }; },
          error => { settled = true; return { error }; },
        );
        // An idle unlock query proves that the new session tried and released
        // a partial acquisition. Existing holders and their heartbeats are excluded.
        await expect.poll(async () => {
          const { rows } = await observer.query(`SELECT pid FROM pg_stat_activity
            WHERE datname = current_database() AND application_name = $1
              AND NOT (pid = ANY($2::int[])) AND state = 'idle'
              AND query = 'SELECT pg_advisory_unlock_all()'`, [application, existing]);
          return rows.length;
        }, { timeout: 3_000, message: "The waiter has completed an unsuccessful acquisition" }).toBe(1);
        expect(settled).toBe(false);
        const released = reservations[0];
        await released.release();
        const result = await waiting;
        if ("error" in result) throw result.error;
        expect(result.value.accounts).toEqual(released.accounts);
      } finally {
        // Settle before releasing holders: no late acquisition may escape cleanup.
        await waiting;
        await Promise.all(reservations.map(item => item.release()));
        await observer.end();
      }
    });
  }
});
