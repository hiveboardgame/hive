import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { access, chmod, mkdtemp, readFile, rm, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { Client } from "pg";

const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, "../..");

for (const scenario of ["signal", "late-output", "startup-interrupt"]) {
  test(`runner cleanup survives ${scenario}`, { timeout: 35_000 }, async () => {
    assert.ok(process.env.TEST_DATABASE_URL, "select the native test environment before checking runner cleanup");
    const temporary = await mkdtemp(path.join(tmpdir(), "hive-runner-regression-"));
    const application = path.join(temporary, "application.mjs");
    const marker = path.join(temporary, "database-name");
    const sourceSite = path.join(temporary, "site");
    await mkdir(path.join(sourceSite, "pkg"), { recursive: true });
    await writeFile(path.join(sourceSite, "pkg/HiveGame.js"), "");
    await writeFile(path.join(sourceSite, "pkg/HiveGame.wasm"), Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]));
    await writeFile(application, `#!/usr/bin/env node
import { writeFileSync } from "node:fs";
import { spawn } from "node:child_process";
writeFileSync(process.env.HIVE_E2E_SIGNAL_MARKER, new URL(process.env.DATABASE_URL).pathname.slice(1));
if (${JSON.stringify(scenario)} === "late-output") {
  spawn(process.execPath, ["-e", "setTimeout(() => process.stderr.write('late application stderr' + String.fromCharCode(10)), 800)"], { stdio: ["ignore", "inherit", "inherit"] }).unref();
}
if (${JSON.stringify(scenario)} === "startup-interrupt") {
  setInterval(() => {}, 1000);
} else {
  process.kill(process.pid, "SIGKILL");
}
`);
    await chmod(application, 0o700);
    const database = new Client({ connectionString: process.env.TEST_DATABASE_URL, connectionTimeoutMillis: 5_000, query_timeout: 5_000 });
    let child;
    let timeout;
    let output = "";
    let timedOut = false;
    let ownedDatabase;
    try {
      await database.connect();
      child = spawn(process.execPath, [path.join(directory, "run.mjs")], {
        cwd: directory,
        env: {
          ...process.env,
          PLAYWRIGHT_APP_BIN: application,
          PLAYWRIGHT_FIXTURE_BIN: application,
          PLAYWRIGHT_SITE_ROOT: sourceSite,
          HIVE_E2E_SIGNAL_MARKER: marker,
        },
        stdio: ["ignore", "pipe", "pipe"],
      });
      child.stdout.on("data", chunk => { output += chunk.toString(); });
      child.stderr.on("data", chunk => { output += chunk.toString(); });
      timeout = setTimeout(() => { timedOut = true; child.kill("SIGKILL"); }, 20_000);
      const closed = once(child, "close");
      if (scenario === "startup-interrupt") {
        // Interrupt only once the fake app is running inside the readiness poll.
        while (child.exitCode === null && child.signalCode === null) {
          if (await access(marker).then(() => true, () => false)) break;
          await new Promise(resolve => setTimeout(resolve, 20));
        }
        child.kill("SIGINT");
      }
      const [code, signal] = await closed;
      clearTimeout(timeout);
      ownedDatabase = (await readFile(marker, "utf8")).trim();
      assert.equal(timedOut, false, "the runner must not wait for another event from its dead application");
      assert.equal(signal, null, "the runner should report failure without needing termination");
      assert.equal(code, scenario === "startup-interrupt" ? 130 : 1, output);
      if (scenario === "startup-interrupt") {
        assert.doesNotMatch(output, /did not become ready/);
      }
      const runId = ownedDatabase.slice("hive_e2e_test_".length);
      if (scenario !== "startup-interrupt") {
        assert.match(output, new RegExp(`test-results/runs/${runId}/server\\.log`), "startup failure identifies this run's application log");
      }
      if (scenario === "late-output") {
        assert.match(await readFile(path.join(directory, "test-results/runs", runId, "server.log"), "utf8"), /^late application stderr$/m);
        assert.doesNotMatch(output, /ERR_STREAM_WRITE_AFTER_END/);
      }
      const { rows } = await database.query("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1) AS present", [ownedDatabase]);
      assert.equal(rows[0].present, false, "the failed run removed its own database");
      await assert.rejects(access(path.join(root, "target/e2e-sites", runId)), { code: "ENOENT" });
    } finally {
      clearTimeout(timeout);
      if (child && child.exitCode === null && child.signalCode === null) {
        const stopped = once(child, "close");
        child.kill("SIGKILL");
        await stopped;
      }
      // Remove only this attempt's resources after assertions can observe a regression.
      ownedDatabase ??= await readFile(marker, "utf8").then(value => value.trim(), () => undefined);
      try {
        if (ownedDatabase && /^hive_e2e_test_[a-f0-9]{32}$/.test(ownedDatabase)) {
          await database.query(`DROP DATABASE IF EXISTS "${ownedDatabase}" WITH (FORCE)`);
          await rm(path.join(root, "target/e2e-sites", ownedDatabase.slice("hive_e2e_test_".length)), { recursive: true, force: true });
        }
      } finally {
        await database.end();
        await rm(temporary, { recursive: true, force: true });
      }
    }
  });
}

test("runner diagnoses an unset test database before starting resources", { timeout: 10_000 }, async () => {
  const environment = { ...process.env };
  delete environment.TEST_DATABASE_URL;
  const child = spawn(process.execPath, [path.join(directory, "run.mjs")], {
    cwd: directory,
    env: environment,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let output = "";
  child.stdout.on("data", chunk => { output += chunk.toString(); });
  child.stderr.on("data", chunk => { output += chunk.toString(); });
  const [code, signal] = await once(child, "close");
  assert.equal(signal, null);
  assert.equal(code, 1, output);
  assert.match(output, /TEST_DATABASE_URL/);
  assert.doesNotMatch(output, /Invalid URL/);
});
