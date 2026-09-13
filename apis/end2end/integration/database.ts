import { test as base } from "@playwright/test";
import { Client } from "pg";
import { randomUUID } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

const repo = path.resolve(__dirname, "../../..");
export const seed = path.join(repo, "db/testware/2026-09-05-000000_e2e_users");

export const test = base.extend<{}, { databaseUrl: string }>({
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

export { expect } from "@playwright/test";
