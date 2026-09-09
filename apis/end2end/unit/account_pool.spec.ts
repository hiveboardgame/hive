import { test, expect } from "@playwright/test";
import { mock } from "node:test";
import { Client } from "pg";
import { publicTimeControl } from "../support/accounts/catalog";
import { reserveAccounts } from "../support/accounts/pool";
import type { AccountReservation } from "../support/accounts/reservation";

const accounts = Array.from({ length: 8 }, (_, index) => ({ id: String(index + 1), username: `user_${index + 1}` }));
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

async function reserve(count: 1 | 2, timeout = 0) {
  const reservation = await reserveAccounts(count, { connectionString: "postgres://localhost/unit", timeout });
  reservations.push(reservation);
  return reservation;
}

test("single users occupy only the primary pool and have distinct public queues", async () => {
  const users = [];
  for (let index = 0; index < 4; index++) users.push(await reserve(1));
  expect(users.every(user => user.accounts.length === 1)).toBe(true);
  expect(new Set(users.map(user => user.accounts[0].username))).toEqual(new Set(["user_1", "user_3", "user_5", "user_7"]));
  expect(new Set(users.map(user => publicTimeControl(user.accounts[0]))).size).toBe(4);
  await expect(reserve(1)).rejects.toThrow("required pool is occupied");
  expect(locks.size).toBe(4);
});

test("pairs take independent primary and partner accounts without fixed partners", async () => {
  locks.set(2, "external");
  locks.set(3, "external");
  locks.set(5, "external");
  locks.set(7, "external");
  const pair = await reserve(2);
  expect(pair.accounts[0].username).toBe("user_1");
  expect(["user_4", "user_6", "user_8"]).toContain(pair.accounts[1].username);
  expect(pair.accounts).toHaveLength(2);
});

test("single users and pairs never share accounts", async () => {
  const single = await reserve(1);
  const pairs = [];
  for (let index = 0; index < 3; index++) pairs.push(await reserve(2));
  expect(new Set([single, ...pairs].flatMap(value => value.accounts.map(account => account.id))).size).toBe(7);
  await expect(reserve(2)).rejects.toThrow("required pool is occupied");
  await single.release();
  const pair = await reserve(2);
  expect(pair.accounts[0]).toEqual(single.accounts[0]);
});

test("a waiting pair releases its primary lock so a single user can acquire it", async () => {
  for (const key of [2, 4, 6, 8]) locks.set(key, "external");
  const waiting = reserve(2, 1_000);
  // Poll the model until the initial attempt released its primary account.
  await expect.poll(() => unlocked).toBeGreaterThan(0);
  const single = await reserve(1);
  locks.delete(4);
  const pair = await waiting;
  expect(pair.accounts[0]).not.toEqual(single.accounts[0]);
  expect(pair.accounts[1].username).toBe("user_4");
});

test("exhaustion releases partial locks and closes the connection", async () => {
  for (const key of [2, 4, 6, 8]) locks.set(key, "external");
  await expect(reserve(2)).rejects.toThrow("required pool is occupied");
  expect([...locks.values()]).toEqual(Array(4).fill("external"));
  expect(ended).toBe(1);
});

test("validation and acquisition errors close sessions without leaking locks", async () => {
  validAccounts = false;
  await expect(reserve(1)).rejects.toThrow("requires verified");
  validAccounts = true;
  failQuery = "pg_try_advisory_lock";
  await expect(reserve(2)).rejects.toThrow("Database connection failed");
  expect(locks.size).toBe(0);
  expect(ended).toBe(2);
});

test("an acquisition error after locking the primary releases that lock", async () => {
  failPartner = true;
  await expect(reserve(2)).rejects.toThrow("Database connection failed");
  expect(locks.size).toBe(0);
  expect(ended).toBe(1);
});

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

test("partners cannot publish into a primary user's Quick Play queue", () => {
  expect(() => publicTimeControl(accounts[1])).toThrow("primary test account");
});
