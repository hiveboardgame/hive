import { writeFile, readFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { once } from "node:events";
import path from "node:path";
import { test, expect } from "./database";
import { reserveAccounts } from "../support/accounts/pool";
import { poolCapacity } from "../support/accounts/catalog";
import type { AccountReservation } from "../support/accounts/reservation";

test.describe("Cross-process ownership", () => {
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
      import { writeFileSync, renameSync } from 'node:fs';
      import { reserveAccounts } from ${JSON.stringify(poolModule)};
      test('hold the last available two-user reservation', async () => {
        const reservation = await reserveAccounts(2);
        writeFileSync(${JSON.stringify(marker + ".tmp")}, JSON.stringify({ pid: process.pid, accounts: reservation.accounts }));
        renameSync(${JSON.stringify(marker + ".tmp")}, ${JSON.stringify(marker)});
        await new Promise(() => {});
      });
    `);
    const reservations: AccountReservation[] = [];
    let ownerPid: number | undefined;
    let child: ReturnType<typeof spawn> | undefined;
    let childOutput = "";
    try {
      for (let i = 0; i < poolCapacity - 1; i++) reservations.push(await reserveAccounts(2, { connectionString: databaseUrl }));
      child = spawn(process.execPath, [path.join(path.dirname(require.resolve("playwright/package.json")), "cli.js"), "test", "--config", config], {
        cwd: directory,
        env: { ...process.env, PLAYWRIGHT_DATABASE_URL: databaseUrl }, stdio: ["ignore", "pipe", "pipe"],
      });
      child.stdout!.on("data", chunk => { childOutput += chunk.toString(); });
      child.stderr!.on("data", chunk => { childOutput += chunk.toString(); });
      await expect.poll(async () => {
        if (child!.exitCode !== null || child!.signalCode !== null) throw new Error(`Child runner exited before acquiring an account: ${childOutput}`);
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
      if (child && child.exitCode === null && child.signalCode === null) {
        const stopped = once(child, "exit");
        child.kill();
        await stopped;
      }
      await Promise.all(reservations.map(item => item.release()));
    }
  });
});
