import { Client } from "pg";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { test, expect, seed } from "./database";
import { usernames } from "../support/accounts/catalog";
import { reserveAccounts } from "../support/accounts/pool";

test.describe("Seed lifecycle", () => {
  test("the consolidated seed rolls back completely and reapplies with stable account identities", async ({ databaseUrl }) => {
    const db = new Client({ connectionString: databaseUrl });
    await db.connect();
    try {
      const identities = async () => (await db.query(`SELECT id, username, password, email,
        normalized_username, admin, email_verified FROM users ORDER BY id`)).rows;
      const original = await identities();
      expect(original.map(account => account.id)).toEqual(
        Array.from({ length: usernames.length + 2 }, (_, i) => `00000000-0000-4000-8000-${(i + 1).toString(16).padStart(12, "0")}`),
      );
      const counts = async () => {
        const { rows } = await db.query(`SELECT username, admin, email_verified,
          (SELECT count(*)::int FROM ratings WHERE user_uid = users.id) AS ratings,
          (SELECT count(*)::int FROM notification_preferences WHERE user_id = users.id) AS preferences
          FROM users ORDER BY id`);
        return rows;
      };
      const expected = [
        { username: "admin_1", admin: true, email_verified: true, ratings: 6, preferences: 1 },
        ...usernames.map(username => ({
          username, admin: false, email_verified: true, ratings: 6, preferences: 1,
        })),
        { username: "SwissByePlayer", admin: false, email_verified: true, ratings: 6, preferences: 1 },
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
});
