import { test, expect } from "@playwright/test";
import { mock } from "node:test";
import { Client } from "pg";
import { poolCapacity, usernames, publicTimeControls, publicTimeControl } from "../support/accounts/catalog";
import { reserveAccounts } from "../support/accounts/pool";
import type { AccountReservation } from "../support/accounts/reservation";

const accounts = usernames.map((username, index) => ({ id: String(index + 1), username }));
const reservations: AccountReservation[] = [];
const locks = new Map<number, Client | "external">();
let failQuery = "";
let failPartner = false;
let validAccounts = true;
let ended = 0;
let unlocked = 0;

// Exercise the real allocator against a session-lock model, with no database.
test.beforeEach(() => {
  locks.clear();
  failQuery = "";
  failPartner = false;
  validAccounts = true;
  ended = 0;
  unlocked = 0;
  mock.method(Client.prototype, "connect", async () => {});
  mock.method(Client.prototype, "end", async function (this: Client) {
    ended++;
    for (const [key, owner] of locks) if (owner === this) locks.delete(key);
    this.emit("end");
  });
  mock.method(Client.prototype, "query", async function (this: Client, sql: string, values?: number[]) {
    if (failQuery && sql.includes(failQuery)) throw new Error("query failed");
    if (sql.includes("FROM users")) return { rows: validAccounts ? accounts : accounts.slice(0, 2) };
    if (sql.includes("pg_try_advisory_lock")) {
      const key = values![1];
      if (failPartner && key % 2 === 0) throw new Error("partner query failed");
      const acquired = !locks.has(key);
      if (acquired) locks.set(key, this);
      return { rows: [{ acquired }] };
    }
    if (sql.includes("pg_advisory_unlock_all")) {
      unlocked++;
      for (const [key, owner] of locks) if (owner === this) locks.delete(key);
    }
    return { rows: [] };
  });
});

test.afterEach(async () => {
  await Promise.all(reservations.splice(0).map(reservation => reservation.release()));
  mock.restoreAll();
});

async function reserve(count: 1 | 2, timeout = 0, publicChallenge = false) {
  const reservation = await reserveAccounts(count, { connectionString: "postgres://localhost/unit", timeout, publicChallenge });
  reservations.push(reservation);
  return reservation;
}

test.describe("Account allocation", () => {
  test("single users occupy only the primary pool", async () => {
    const users = [];
    for (let index = 0; index < poolCapacity; index++) users.push(await reserve(1));
    expect(users.every(user => user.accounts.length === 1)).toBe(true);
    expect(new Set(users.map(user => user.accounts[0].username))).toEqual(new Set(usernames.filter((_, index) => index % 2 === 0)));
    await expect(reserve(1)).rejects.toThrow("required pool is occupied");
    expect(locks.size).toBe(poolCapacity);
  });

  test("public reservations use distinct queues while general reservations use remaining accounts", async () => {
    const publicUsers = [];
    for (let i = 0; i < publicTimeControls.length; i++) publicUsers.push(await reserve(2, 0, true));
    expect(new Set(publicUsers.map(item => publicTimeControl(item.accounts[0])))).toEqual(new Set(publicTimeControls));
    await expect(reserve(2, 0, true)).rejects.toThrow("required pool is occupied");
    const general = await reserve(2);
    expect(Number(general.accounts[0].username.slice(5))).toBeGreaterThan(10);
    expect(() => publicTimeControl(general.accounts[0])).toThrow("queue-owning account");
  });

  test("pairs take independent primary and partner accounts without fixed partners", async () => {
    locks.set(2, "external");
    for (let key = 3; key <= usernames.length; key += 2) locks.set(key, "external");
    const pair = await reserve(2);
    expect(pair.accounts[0].username).toBe("user_1");
    expect(usernames.filter((_, index) => index % 2 === 1 && index > 1)).toContain(pair.accounts[1].username);
    expect(pair.accounts).toHaveLength(2);
  });

  test("single users and pairs never share accounts", async () => {
    const single = await reserve(1);
    const pairs = [];
    for (let index = 0; index < poolCapacity - 1; index++) pairs.push(await reserve(2));
    expect(new Set([single, ...pairs].flatMap(value => value.accounts.map(account => account.id))).size).toBe(poolCapacity * 2 - 1);
    await expect(reserve(2)).rejects.toThrow("required pool is occupied");
    await single.release();
    const pair = await reserve(2);
    expect(pair.accounts[0]).toEqual(single.accounts[0]);
  });

  test("a waiting pair releases its primary lock so a single user can acquire it", async () => {
    for (const key of Array.from({ length: poolCapacity }, (_, index) => index * 2 + 2)) locks.set(key, "external");
    const waiting = reserve(2, 5_000).then(value => ({ value }), error => ({ error }));
    try {
      // Poll the model until the initial attempt released its primary account.
      await expect.poll(() => unlocked).toBeGreaterThan(0);
      const single = await reserve(1);
      locks.delete(4);
      const result = await waiting;
      if ("error" in result) throw result.error;
      const pair = result.value;
      expect(pair.accounts[0]).not.toEqual(single.accounts[0]);
      expect(pair.accounts[1].username).toBe("user_4");
    } finally {
      await waiting;
    }
  });

  test("exhaustion releases partial locks and closes the connection", async () => {
    for (const key of Array.from({ length: poolCapacity }, (_, index) => index * 2 + 2)) locks.set(key, "external");
    await expect(reserve(2)).rejects.toThrow("required pool is occupied");
    expect([...locks.values()]).toEqual(Array(poolCapacity).fill("external"));
    expect(ended).toBe(1);
  });

  test("validation failure closes its session without leaking locks", async () => {
    validAccounts = false;
    await expect(reserve(1)).rejects.toThrow("requires verified");
    expect(locks.size).toBe(0);
    expect(ended).toBe(1);
  });

  test("acquisition failure closes its session without leaking locks", async () => {
    failQuery = "pg_try_advisory_lock";
    await expect(reserve(2)).rejects.toThrow("Database connection failed");
    expect(locks.size).toBe(0);
    expect(ended).toBe(1);
  });

  test("an acquisition error after locking the primary releases that lock", async () => {
    failPartner = true;
    await expect(reserve(2)).rejects.toThrow("Database connection failed");
    expect(locks.size).toBe(0);
    expect(ended).toBe(1);
  });

  test.describe("Ownership loss", () => {
    test("session loss invalidates a reservation and release is idempotent", async () => {
      const reservation = await reserve(1);
      const owner = [...locks.values()][0] as Client;
      let lost: Error | undefined;
      reservation.onLost(error => { lost = error; });
      owner.emit("error", new Error("connection lost"));
      expect(lost?.message).toContain("Lost the database connection");
      expect(() => reservation.assertHeld()).toThrow("Lost the database connection");
      await expect(reservation.state()).rejects.toThrow("Lost the database connection");
      await reservation.release();
      await reservation.release();
      expect(locks.size).toBe(0);
      expect(ended).toBe(1);
    });
  });

  test("partners cannot publish into a primary user's Quick Play queue", () => {
    expect(() => publicTimeControl(accounts[1])).toThrow("primary test account");
  });
});
