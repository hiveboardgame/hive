import { recover } from "./data.ts";

const manifest = process.argv[2];
if (!manifest || !process.env.PLAYWRIGHT_DATABASE_URL) {
  throw new Error("Usage: PLAYWRIGHT_DATABASE_URL=... node support/tournaments/recover.mjs <manifest.json>");
}
await recover(process.env.PLAYWRIGHT_DATABASE_URL, manifest);
console.log("Recovered only the records owned by the tournament manifest.");
