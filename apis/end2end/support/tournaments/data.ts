import { Client } from "pg";
import { randomUUID } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";

export const byeId = "00000000-0000-4000-8000-000000000020";
const lockNamespace = 0x5455524e; // TURN; separate from the gameplay account pool.
export type Account = { id: string; username: string; admin: boolean };
export type Tournament = { id: string; nanoid: string; name: string; organizer: string };
export type Manifest = {
  version: 1; owner: string; lock: number; database: string;
  accounts: Account[]; tournaments: Tournament[];
};
export const description = "A tournament fixture for browser interaction checks. Results are not a scoring oracle.";

function validateManifest(manifest: Manifest) {
  const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
  if (manifest.version !== 1 || !uuid.test(manifest.owner)
    || manifest.lock !== (parseInt(manifest.owner.slice(0, 8), 16) & 0x7fffffff)
    || !Array.isArray(manifest.accounts) || !Array.isArray(manifest.tournaments)) throw new Error("Invalid tournament ownership manifest.");
  const prefix = `tu_${manifest.owner.replaceAll("-", "").slice(0, 12)}_`;
  if (manifest.accounts.some((a, i) => !uuid.test(a.id) || a.id.startsWith("00000000-0000-4000-8000-")
    || a.username !== `${prefix}${i}` || typeof a.admin !== "boolean")
    || new Set(manifest.accounts.map(a => a.id)).size !== manifest.accounts.length
    || manifest.tournaments.some((t, i) => !uuid.test(t.id)
      || t.name !== `UI ${manifest.owner.slice(0, 8)} ${i}`
      || !manifest.accounts.some(a => a.id === t.organizer))) throw new Error("Invalid tournament ownership manifest.");
}

export async function transaction<T>(client: Client, action: () => Promise<T>): Promise<T> {
  await client.query("BEGIN");
  try {
    const value = await action();
    await client.query("COMMIT");
    return value;
  } catch (error) {
    await client.query("ROLLBACK");
    throw error;
  }
}

export class TournamentData {
  readonly client: Client;
  readonly manifest: Manifest;
  readonly manifestPath: string;
  private lost = false;
  private held = false;
  private heartbeat?: ReturnType<typeof setInterval>;
  onLost: () => void = () => {};

  constructor(connectionString: string, manifestPath: string, manifest?: Manifest) {
    this.client = new Client({ connectionString, connectionTimeoutMillis: 5000, query_timeout: 5000 });
    const owner = randomUUID();
    this.manifest = manifest ?? { version: 1, owner, lock: parseInt(owner.slice(0, 8), 16) & 0x7fffffff,
      database: "", accounts: [], tournaments: [] };
    this.manifestPath = manifestPath;
    this.client.on("error", () => this.lose());
    this.client.on("end", () => { if (this.held) this.lose(); });
  }

  private lose() { this.lost = true; this.onLost(); }
  assertHeld() { if (!this.held || this.lost) throw new Error("Tournament fixture ownership was lost; use its manifest to recover."); }

  async open() {
    await this.client.connect();
    const { rows } = await this.client.query("SELECT current_database() AS name");
    if (this.manifest.database && this.manifest.database !== rows[0].name) throw new Error("Manifest belongs to another database.");
    this.manifest.database = rows[0].name;
    const result = await this.client.query("SELECT pg_try_advisory_lock($1, $2) AS acquired", [lockNamespace, this.manifest.lock]);
    if (!result.rows[0].acquired) throw new Error("Tournament fixture has an active owner; recovery refused.");
    this.held = true;
    this.heartbeat = setInterval(() => { void this.client.query("SELECT 1").catch(() => this.lose()); }, 5000);
    await this.save();
  }

  async save() {
    await mkdir(path.dirname(this.manifestPath), { recursive: true });
    await writeFile(`${this.manifestPath}.tmp`, JSON.stringify(this.manifest, null, 2));
    await rename(`${this.manifestPath}.tmp`, this.manifestPath);
  }

  async accounts(count: number, admin = false): Promise<Account[]> {
    this.assertHeld();
    const accounts = Array.from({ length: count }, (_, n) => ({ id: randomUUID(),
      username: `tu_${this.manifest.owner.replaceAll("-", "").slice(0, 12)}_${this.manifest.accounts.length + n}`,
      admin }));
    this.manifest.accounts.push(...accounts);
    await this.save(); // Journal identities before any insert, including failed setup.
    await transaction(this.client, async () => {
      for (const account of accounts) {
        const result = await this.client.query(`INSERT INTO users
          (id, username, normalized_username, email, password, admin, email_verified, created_at, updated_at)
          SELECT $1, $2, $2, $2 || '@example.test', password, $3, true, now(), now()
          FROM users WHERE id = '00000000-0000-4000-8000-000000000002' AND username = 'user_1'`,
        [account.id, account.username, account.admin]);
        if (result.rowCount !== 1) throw new Error("Apply the consolidated testware seed before tournament tests.");
        await this.client.query(`INSERT INTO ratings
          (user_uid, played, won, lost, draw, rating, deviation, volatility, created_at, updated_at, speed)
          SELECT $1, 0, 0, 0, 0, 1500, 500, 0.09, now(), now(), speed
          FROM unnest(ARRAY['Bullet','Blitz','Rapid','Classic','Correspondence','Puzzle']) AS speed`, [account.id]);
        await this.client.query("INSERT INTO notification_preferences (user_id) VALUES ($1)", [account.id]);
      }
    });
    return accounts;
  }

  async plan(organizer: Account): Promise<Tournament> {
    this.assertHeld();
    if (!this.manifest.accounts.some(a => a.id === organizer.id)) throw new Error("Organizer is not owned by this fixture.");
    const id = randomUUID();
    const tournament = { id, nanoid: id.replaceAll("-", "").slice(0, 11),
      name: `UI ${this.manifest.owner.slice(0, 8)} ${this.manifest.tournaments.length}`, organizer: organizer.id };
    this.manifest.tournaments.push(tournament);
    await this.save();
    return tournament;
  }

  async seed(organizer: Account, options: {
    players?: Account[]; status?: "NotStarted" | "InProgress" | "Finished";
    inviteOnly?: boolean; seats?: number; minSeats?: number; lower?: number; upper?: number;
    mode?: "DoubleRoundRobin" | "DoubleSwiss";
  } = {}): Promise<Tournament> {
    const tournament = await this.plan(organizer);
    const players = options.players ?? [];
    if (players.some(p => !this.manifest.accounts.some(a => a.id === p.id))) throw new Error("Participant is not owned by this fixture.");
    await transaction(this.client, async () => {
      await this.client.query(`INSERT INTO tournaments
        (id,nanoid,name,description,scoring,tiebreaker,seats,min_seats,rounds,invite_only,mode,time_mode,
         time_base,time_increment,band_lower,band_upper,start_mode,status,started_at,created_at,updated_at)
        VALUES ($1,$2,$3,$4,'Game',ARRAY['RawPoints','HeadToHead','WinsAsBlack','SonnebornBerger'],
          $5,$6,1,$7,$8,'Real Time',600,10,$9,$10,'Manual',$11,
          CASE WHEN $11 = 'NotStarted' THEN NULL ELSE now() END,now(),now())`,
      [tournament.id, tournament.nanoid, tournament.name, description, options.seats ?? 4, options.minSeats ?? 2,
        options.inviteOnly ?? false, options.mode ?? "DoubleRoundRobin", options.lower ?? null, options.upper ?? null,
        options.status ?? "NotStarted"]);
      await this.client.query("INSERT INTO tournaments_organizers VALUES ($1,$2)", [tournament.id, organizer.id]);
      for (const player of players) await this.client.query("INSERT INTO tournaments_users VALUES ($1,$2)", [tournament.id, player.id]);
    });
    return tournament;
  }

  async createdThroughUI(planned: Tournament): Promise<Tournament> {
    this.assertHeld();
    const { rows } = await this.client.query(`SELECT t.id,t.nanoid,t.name,o.organizer_id AS organizer
      FROM tournaments t JOIN tournaments_organizers o ON o.tournament_id=t.id
      WHERE t.name=$1 AND o.organizer_id=$2`, [planned.name, planned.organizer]);
    if (rows.length !== 1) throw new Error("UI-created tournament has not persisted.");
    Object.assign(planned, rows[0]);
    await this.save();
    return planned;
  }

  async games(tournament: Tournament): Promise<{ id: string; nanoid: string }[]> {
    this.assertHeld();
    return (await this.client.query("SELECT id,nanoid FROM games WHERE tournament_id=$1 ORDER BY nanoid", [tournament.id])).rows;
  }

  async requireBye() {
    const result = await this.client.query(`SELECT id FROM users WHERE id=$1 AND username='SwissByePlayer'
      AND normalized_username='swissbyeplayer' AND NOT admin AND NOT deleted
      AND (SELECT count(*) FROM ratings WHERE user_uid=$1)=6`, [byeId]);
    if (result.rowCount !== 1) throw new Error("Update the consolidated testware seed with SwissByePlayer before Swiss UI checks.");
  }

  async cleanup() {
    this.assertHeld();
    validateManifest(this.manifest);
    const accountIds = this.manifest.accounts.map(a => a.id);
    await transaction(this.client, async () => {
      // Recover a Create action even if the runner died before recording its returned ID.
      const found = await this.client.query(`SELECT t.id,t.nanoid,t.name,o.organizer_id AS organizer
        FROM tournaments t JOIN tournaments_organizers o ON o.tournament_id=t.id
        WHERE t.name=ANY($1::text[]) FOR UPDATE OF t`, [this.manifest.tournaments.map(t => t.name)]);
      for (const row of found.rows) {
        const planned = this.manifest.tournaments.find(t => t.name === row.name);
        if (!planned || planned.organizer !== row.organizer) throw new Error("Foreign tournament ownership; cleanup refused.");
        Object.assign(planned, row);
      }
      const ids = this.manifest.tournaments.map(t => t.id);
      const identities = await this.client.query("SELECT id,name FROM tournaments WHERE id=ANY($1::uuid[])", [ids]);
      if (identities.rows.some(row => !this.manifest.tournaments.some(t => t.id === row.id && t.name === row.name))) {
        throw new Error("Foreign tournament identity; cleanup refused.");
      }
      const allowed = [...accountIds, byeId];
      const foreign = await this.client.query(`SELECT 1 FROM tournaments_organizers WHERE
          (tournament_id=ANY($1::uuid[]) AND NOT organizer_id=ANY($2::uuid[])) OR
          (organizer_id=ANY($2::uuid[]) AND NOT tournament_id=ANY($1::uuid[]))
        UNION ALL SELECT 1 FROM tournaments_users WHERE
          (tournament_id=ANY($1::uuid[]) AND NOT user_id=ANY($3::uuid[])) OR
          (user_id=ANY($2::uuid[]) AND NOT tournament_id=ANY($1::uuid[]))
        UNION ALL SELECT 1 FROM tournaments_invitations WHERE
          (tournament_id=ANY($1::uuid[]) AND NOT invitee_id=ANY($3::uuid[])) OR
          (invitee_id=ANY($2::uuid[]) AND NOT tournament_id=ANY($1::uuid[]))
        UNION ALL SELECT 1 FROM games WHERE
          (tournament_id=ANY($1::uuid[]) AND (NOT white_id=ANY($3::uuid[]) OR NOT black_id=ANY($3::uuid[]))) OR
          ((white_id=ANY($2::uuid[]) OR black_id=ANY($2::uuid[])) AND
            (tournament_id IS NULL OR NOT tournament_id=ANY($1::uuid[])))`, [ids, accountIds, allowed]);
      if (foreign.rowCount) throw new Error("Foreign participant or game; cleanup refused.");
      for (const account of this.manifest.accounts) {
        const actual = await this.client.query("SELECT username,email,admin FROM users WHERE id=$1 FOR UPDATE", [account.id]);
        if (actual.rows.some(a => a.username !== account.username || a.email !== `${account.username}@example.test` || a.admin !== account.admin)) {
          throw new Error("Foreign account identity; cleanup refused.");
        }
      }
      // games.tournament_id has no foreign key: explicitly delete games first.
      await this.client.query("DELETE FROM games WHERE tournament_id=ANY($1::uuid[])", [ids]);
      await this.client.query("DELETE FROM tournaments WHERE id=ANY($1::uuid[])", [ids]);
      await this.client.query("DELETE FROM users WHERE id=ANY($1::uuid[])", [accountIds]);
      const remaining = await this.client.query(`SELECT 1 FROM users WHERE id=ANY($1::uuid[])
        UNION ALL SELECT 1 FROM ratings WHERE user_uid=ANY($1::uuid[])
        UNION ALL SELECT 1 FROM notification_preferences WHERE user_id=ANY($1::uuid[])
        UNION ALL SELECT 1 FROM tournaments WHERE id=ANY($2::uuid[])
        UNION ALL SELECT 1 FROM games WHERE tournament_id=ANY($2::uuid[])
        UNION ALL SELECT 1 FROM schedules WHERE tournament_id=ANY($2::uuid[])
        UNION ALL SELECT 1 FROM chat_channels WHERE tournament_id=ANY($2::uuid[])`, [accountIds, ids]);
      if (remaining.rowCount) throw new Error("Tournament fixture cleanup left owned records.");
    });
    await this.save();
  }

  async close() {
    clearInterval(this.heartbeat);
    this.held = false;
    await this.client.end();
  }
}

export async function recover(connectionString: string, manifestPath: string) {
  const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as Manifest;
  validateManifest(manifest);
  const data = new TournamentData(connectionString, manifestPath, manifest);
  try { await data.open(); await data.cleanup(); } finally { await data.close(); }
}
