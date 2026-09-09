import { test as base, expect } from "@playwright/test";
import { Client } from "pg";
import { randomUUID } from "node:crypto";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { once } from "node:events";
import path from "node:path";
import { reserveAccounts } from "../support/accounts/pool";
import { publicTimeControl } from "../support/accounts/catalog";
import type { AccountReservation } from "../support/accounts/reservation";

const repo = path.resolve(__dirname, "../../..");
const seed = path.join(repo, "db/testware/2026-09-05-000000_e2e_users");

const test = base.extend<{}, { databaseUrl: string }>({
  databaseUrl: [async ({}, use) => {
    const source = process.env.PLAYWRIGHT_DATABASE_URL;
    if (!source) throw new Error("Set PLAYWRIGHT_DATABASE_URL to a test PostgreSQL connection with CREATE DATABASE permission.");
    const name = `hive_pool_check_${randomUUID().replaceAll("-", "")}`;
    const url = new URL(source);
    url.pathname = `/${name}`;
    const admin = new Client({ connectionString: source });
    await admin.connect();
    try {
      await admin.query(`CREATE DATABASE "${name}"`);
      const db = new Client({ connectionString: url.href });
      await db.connect();
      try {
        const migrations = (await readdir(path.join(repo, "db/migrations"), { withFileTypes: true }))
          .filter(entry => entry.isDirectory()).map(entry => entry.name).sort();
        for (const migration of migrations) {
          await db.query(await readFile(path.join(repo, "db/migrations", migration, "up.sql"), "utf8"));
        }
        await db.query(await readFile(path.join(seed, "up.sql"), "utf8"));
      } finally {
        await db.end();
      }
      await use(url.href);
    } finally {
      await admin.query(`DROP DATABASE IF EXISTS "${name}" WITH (FORCE)`);
      await admin.end();
    }
  }, { scope: "worker", timeout: 60_000 }],
});

test("the consolidated seed rolls back completely and reapplies with stable account identities", async ({ databaseUrl }) => {
  const db = new Client({ connectionString: databaseUrl });
  await db.connect();
  try {
    const identities = async () => (await db.query(`SELECT id, username, password, email,
      normalized_username, admin, email_verified FROM users ORDER BY username`)).rows;
    const original = await identities();
    expect(original.map(account => account.id)).toEqual(
      Array.from({ length: 9 }, (_, i) => `00000000-0000-4000-8000-${String(i + 1).padStart(12, "0")}`),
    );
    const counts = async () => {
      const { rows } = await db.query(`SELECT username, admin, email_verified,
        (SELECT count(*)::int FROM ratings WHERE user_uid = users.id) AS ratings,
        (SELECT count(*)::int FROM notification_preferences WHERE user_id = users.id) AS preferences
        FROM users ORDER BY username`);
      return rows;
    };
    const expected = [
      { username: "admin_1", admin: true, email_verified: true, ratings: 6, preferences: 1 },
      ...Array.from({ length: 8 }, (_, i) => ({
        username: `user_${i + 1}`, admin: false, email_verified: true, ratings: 6, preferences: 1,
      })),
    ];
    expect(await counts()).toEqual(expected);
    await db.query(await readFile(path.join(seed, "down.sql"), "utf8"));
    expect(await counts()).toEqual([]);
    for (const table of ["ratings", "notification_preferences"]) {
      expect((await db.query(`SELECT count(*)::int AS count FROM ${table}`)).rows[0].count).toBe(0);
    }
    await expect(reserveAccounts(2, { connectionString: databaseUrl })).rejects.toThrow("requires verified");
    await db.query(await readFile(path.join(seed, "up.sql"), "utf8"));
    expect(await counts()).toEqual(expected);
    expect(await identities()).toEqual(original);
  } finally {
    await db.end();
  }
});

for (const count of [1, 2] as const) {
  test(`${count}-user reservations are exclusive, wait when full, and become reusable after release`, async ({ databaseUrl }) => {
    const reservations: AccountReservation[] = [];
    try {
      for (let i = 0; i < 4; i++) reservations.push(await reserveAccounts(count, { connectionString: databaseUrl }));
      expect(new Set(reservations.flatMap(item => item.accounts.map(account => account.username))).size).toBe(4 * count);
      expect(new Set(reservations.map(item => publicTimeControl(item.accounts[0]))).size).toBe(4);
      await expect(reserveAccounts(count, { connectionString: databaseUrl, timeout: 200 })).rejects.toThrow("required pool is occupied");
      let acquired = false;
      const waiting = reserveAccounts(count, { connectionString: databaseUrl, timeout: 5_000 }).then(value => {
        acquired = true;
        reservations.push(value);
        return value;
      });
      await new Promise(resolve => setTimeout(resolve, 150));
      expect(acquired).toBe(false);
      const released = reservations[0];
      await released.release();
      expect((await waiting).accounts).toEqual(released.accounts);
    } finally {
      await Promise.all(reservations.map(item => item.release()));
    }
  });
}

test("separate Playwright processes share locks and a killed owner releases its pair", async ({ databaseUrl }, testInfo) => {
  const directory = testInfo.outputPath("child");
  const { mkdir } = await import("node:fs/promises");
  await mkdir(directory, { recursive: true });
  const poolModule = path.resolve(__dirname, "../support/accounts/pool.ts");
  const marker = path.join(directory, "acquired.json");
  const config = path.join(directory, "playwright.config.ts");
  await writeFile(config, `export default {
    testDir: '.', testMatch: 'owner.spec.ts', workers: 1, timeout: 30000, reporter: 'line',
    outputDir: ${JSON.stringify(path.join(directory, "results"))},
  };`);
  await writeFile(path.join(directory, "owner.spec.ts"), `
    import { test } from '@playwright/test';
    import { writeFileSync } from 'node:fs';
    import { reserveAccounts } from ${JSON.stringify(poolModule)};
    test('hold the last available two-user reservation', async () => {
      const reservation = await reserveAccounts(2);
      writeFileSync(${JSON.stringify(marker)}, JSON.stringify({ pid: process.pid, accounts: reservation.accounts }));
      await new Promise(() => {});
    });
  `);
  const reservations: AccountReservation[] = [];
  let ownerPid: number | undefined;
  let child: ReturnType<typeof spawn> | undefined;
  let childOutput = "";
  try {
    for (let i = 0; i < 3; i++) reservations.push(await reserveAccounts(2, { connectionString: databaseUrl }));
    child = spawn(process.execPath, [path.join(path.dirname(require.resolve("playwright/package.json")), "cli.js"), "test", "--config", config], {
      cwd: directory,
      env: { ...process.env, PLAYWRIGHT_DATABASE_URL: databaseUrl }, stdio: ["ignore", "pipe", "pipe"],
    });
    child.stdout!.on("data", chunk => { childOutput += chunk.toString(); });
    child.stderr!.on("data", chunk => { childOutput += chunk.toString(); });
    await expect.poll(async () => {
      if (child!.exitCode !== null) throw new Error(`Child runner exited before acquiring an account: ${childOutput}`);
      return readFile(marker, "utf8").catch(() => "");
    }, { timeout: 10_000 }).not.toBe("");
    const owner = JSON.parse(await readFile(marker, "utf8"));
    ownerPid = owner.pid;
    expect(reservations.flatMap(item => item.accounts.map(account => account.username)))
      .not.toContain(owner.accounts[0].username);
    await expect(reserveAccounts(2, { connectionString: databaseUrl, timeout: 100 })).rejects.toThrow("required pool is occupied");
    process.kill(ownerPid!, "SIGKILL");
    ownerPid = undefined;
    const next = await reserveAccounts(2, { connectionString: databaseUrl, timeout: 5_000 });
    reservations.push(next);
    expect(next.accounts).toEqual(owner.accounts);
  } finally {
    await testInfo.attach("Child runner output", { body: childOutput, contentType: "text/plain" });
    if (ownerPid) { try { process.kill(ownerPid, "SIGKILL"); } catch {} }
    if (child && child.exitCode === null) {
      const stopped = once(child, "exit");
      child.kill();
      await stopped;
    }
    await Promise.all(reservations.map(item => item.release()));
  }
});

test("losing a database session notifies the fixture and invalidates the reservation", async ({ databaseUrl }) => {
  const reservation = await reserveAccounts(2, { connectionString: databaseUrl });
  const db = new Client({ connectionString: databaseUrl });
  await db.connect();
  let lost: Error | undefined;
  const unsubscribe = reservation.onLost(error => { lost = error; });
  try {
    await db.query(`SELECT pg_terminate_backend(pid) FROM pg_stat_activity
      WHERE datname = current_database() AND application_name = $1`, [`hive-playwright-${process.pid}`]);
    await expect.poll(() => lost?.message).toContain("Lost the database connection");
    expect(() => reservation.assertHeld()).toThrow("Lost the database connection");
    await expect(reservation.state()).rejects.toThrow("Lost the database connection");
  } finally {
    unsubscribe();
    await reservation.release();
    await db.end();
  }
});

test("connection failures do not disclose credentials", async () => {
  await expect(reserveAccounts(2, { connectionString: "postgres://secret_user:secret_password@127.0.0.1:1/missing" }))
    .rejects.toThrow(/^Cannot connect to PLAYWRIGHT_DATABASE_URL; check test database access\.$/);
});
