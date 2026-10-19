import { execFile, spawn } from "node:child_process";
import { randomBytes, randomUUID } from "node:crypto";
import { createWriteStream } from "node:fs";
import { access, cp, mkdir, realpath, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { once } from "node:events";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Client } from "pg";
import { promisify } from "node:util";

const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, "../..");
const sourceSite = path.resolve(root, process.env.PLAYWRIGHT_SITE_ROOT ?? "target/site");
const target = process.env.CARGO_TARGET_DIR ?? path.join(root, ".cargo/target");
const application = process.env.PLAYWRIGHT_APP_BIN ?? path.join(target, "debug/apis");
const fixtures = process.env.PLAYWRIGHT_FIXTURE_BIN ?? path.join(target, "debug/hive-e2e");
const runId = randomUUID().replaceAll("-", "");
const databaseName = `hive_e2e_test_${runId}`;
const site = path.join(root, "target/e2e-sites", runId);
const logDirectory = path.join(directory, "test-results/runs", runId);
const logDisplay = `test-results/runs/${runId}/server.log`;
let applicationProcess;
let testProcess;
let interrupted = false;
let createdDatabase = false;
let controlURL;
let applicationClosed;
let serverLog;
let exitCode = 1;

const interrupt = () => {
  interrupted = true;
  testProcess?.kill("SIGINT");
};
process.on("SIGINT", interrupt);
process.on("SIGTERM", interrupt);

try {
  if (process.env.HIVE_WORKTREE_ROOT && await realpath(process.env.HIVE_WORKTREE_ROOT) !== await realpath(root)) {
    throw new Error("Leave the other Hive instance shell before running this worktree's tests.");
  }
  if (!process.env.TEST_DATABASE_URL) {
    throw new Error("TEST_DATABASE_URL must select this instance's dedicated local test database.");
  }
  controlURL = new URL(process.env.TEST_DATABASE_URL);
  if (!["postgres:", "postgresql:"].includes(controlURL.protocol)
      || !["localhost", "127.0.0.1", "[::1]"].includes(controlURL.hostname)
      || controlURL.search || controlURL.hash
      || !controlURL.pathname.split(/[-_/]/).includes("test")) {
    throw new Error("TEST_DATABASE_URL must select this instance's dedicated local test database.");
  }
  await Promise.all([access(application), access(fixtures), access(path.join(sourceSite, "pkg/HiveGame.js"))]);
  // Keep the running watcher's assets untouched and remove only non-executable symbols.
  // Leptos's debug name section alone can exceed 250 MB and exhaust WebKit on startup.
  const sourceWasm = path.join(sourceSite, "pkg/HiveGame.wasm");
  await cp(sourceSite, site, { recursive: true, filter: source => source !== sourceWasm });
  try {
    await promisify(execFile)("llvm-strip", ["--strip-all", sourceWasm, "-o", path.join(site, "pkg/HiveGame.wasm")]);
  } catch (error) {
    if (error.code === "ENOENT") throw new Error("llvm-strip is required for browser tests. Install LLVM and add its bin directory to PATH, or use the flake dev shell.");
    throw error;
  }
  const administrator = new Client({ connectionString: controlURL.href, connectionTimeoutMillis: 5000 });
  try {
    await administrator.connect();
    // This identifier is generated locally and belongs to this run alone.
    await administrator.query(`CREATE DATABASE "${databaseName}"`);
    createdDatabase = true;
  } finally { await administrator.end(); }
  const databaseURL = new URL(controlURL);
  databaseURL.pathname = `/${databaseName}`;
  const port = await availablePort();
  const baseURL = `http://127.0.0.1:${port}`;
  await mkdir(logDirectory, { recursive: true });
  serverLog = createWriteStream(path.join(logDirectory, "server.log"));
  const appEnvironment = {
    ...process.env,
    DATABASE_URL: databaseURL.href,
    APP_ORIGIN: baseURL,
    EMAIL_BASE_URL: baseURL,
    COOKIE_SECRET_KEY: randomBytes(64).toString("hex"),
    JWT_SECRET_KEY: randomBytes(64).toString("hex"),
    LEPTOS_SITE_ADDR: `127.0.0.1:${port}`,
    LEPTOS_SITE_ROOT: site,
    WS_TELEMETRY_INTERVAL_SECS: "0",
    VAPID_PRIVATE_KEY: "",
    LETTERMINT_API_KEY: "",
  };
  delete appEnvironment.APP_DEV_ORIGINS;
  delete appEnvironment.LEPTOS_WATCH;
  applicationProcess = spawn(application, [], { cwd: root, env: appEnvironment, stdio: ["ignore", "pipe", "pipe"] });
  // Record close immediately: exit can precede the last stdout/stderr data,
  // and startup failure may reach finally after the process has already closed.
  applicationClosed = new Promise(resolve => applicationProcess.once("close", resolve));
  let startupError;
  applicationProcess.on("error", error => { startupError = error; });
  const redact = chunk => chunk.toString().replaceAll(databaseURL.href, "[browser test database]");
  applicationProcess.stdout.on("data", chunk => serverLog.write(redact(chunk)));
  applicationProcess.stderr.on("data", chunk => serverLog.write(redact(chunk)));
  const deadline = Date.now() + 90_000;
  let healthy = false;
  while (!healthy && Date.now() < deadline && !interrupted) {
    if (startupError) throw startupError;
    if (applicationProcess.exitCode !== null || applicationProcess.signalCode !== null) throw new Error(`The browser-test application exited; see ${logDisplay}.`);
    try {
      const response = await fetch(`${baseURL}/health/ready`, { signal: AbortSignal.timeout(1000) });
      healthy = response.ok;
    } catch { /* The server applies migrations before binding its socket. */ }
    if (!healthy) await new Promise(resolve => setTimeout(resolve, 200));
  }
  if (interrupted) {
    exitCode = 130;
  } else {
    if (!healthy) throw new Error(`The isolated app did not become ready; see ${logDisplay}.`);
    console.log(`Browser-test application: ${baseURL} (empty, run-owned database; logs: ${logDisplay})`);
    testProcess = spawn(process.execPath, [path.join(directory, "node_modules/@playwright/test/cli.js"), "test", ...process.argv.slice(2)], {
      cwd: directory,
      stdio: "inherit",
      env: { ...process.env, PLAYWRIGHT_RUN_ID: runId, PLAYWRIGHT_BASE_URL: baseURL, PLAYWRIGHT_DATABASE_URL: databaseURL.href, PLAYWRIGHT_FIXTURE_BIN: fixtures },
    });
    const [code] = await once(testProcess, "exit");
    exitCode = interrupted ? 130 : code ?? 1;
  }
} catch (error) {
  // Database errors can contain credentials; surface a local diagnostic without connection strings.
  const detail = error instanceof Error ? error.message.replace(/postgres(?:ql)?:\/\/\S+/g, "[database]") : String(error);
  console.error(`Browser test run failed: ${detail}`);
} finally {
  if (applicationProcess?.pid && applicationProcess.exitCode === null && applicationProcess.signalCode === null) {
    applicationProcess.kill("SIGTERM");
    const deadline = setTimeout(() => applicationProcess.kill("SIGKILL"), 5000);
    try { await applicationClosed; } finally { clearTimeout(deadline); }
  }
  await applicationClosed;
  serverLog?.end();
  if (createdDatabase) {
    const administrator = new Client({ connectionString: controlURL.href, connectionTimeoutMillis: 5000 });
    try {
      await administrator.connect();
      await administrator.query(`DROP DATABASE "${databaseName}" WITH (FORCE)`);
    } catch {
      console.error(`Could not remove run-owned database ${databaseName}; remove it after checking for an interrupted test run.`);
      exitCode = 1;
    } finally { await administrator.end(); }
  }
  await rm(site, { recursive: true, force: true });
  process.removeListener("SIGINT", interrupt);
  process.removeListener("SIGTERM", interrupt);
}
process.exitCode = exitCode;

async function availablePort() {
  const reservation = createServer();
  reservation.listen(0, "127.0.0.1");
  await once(reservation, "listening");
  const address = reservation.address();
  if (!address || typeof address === "string") throw new Error("Could not reserve a local test port.");
  const port = address.port;
  await new Promise((resolve, reject) => reservation.close(error => error ? reject(error) : resolve()));
  return port;
}
