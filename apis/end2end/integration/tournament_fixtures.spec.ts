import { Client } from "pg";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";
import { test, expect } from "./database";
import { TournamentData, transaction } from "../support/tournaments/data";

test.describe("Tournament fixture ownership", () => {
  test("rollback removes partial setup without touching the consolidated seed", async ({ databaseUrl }, testInfo) => {
    const data = new TournamentData(databaseUrl, testInfo.outputPath("owner.json"));
    await data.open();
    try {
      // Fail after account insertion but before dependent records can complete.
      await data.client.query(`CREATE FUNCTION reject_tournament_rating() RETURNS trigger LANGUAGE plpgsql AS
        $$ BEGIN RAISE EXCEPTION 'injected rating failure'; END $$`);
      await data.client.query("CREATE TRIGGER fail_rating BEFORE INSERT ON ratings FOR EACH ROW EXECUTE FUNCTION reject_tournament_rating()");
      await expect(data.accounts(2)).rejects.toThrow("injected rating failure");
      expect((await data.client.query("SELECT count(*)::int AS n FROM users")).rows[0].n).toBe(32);
      expect((await data.client.query("SELECT count(*)::int AS n FROM ratings")).rows[0].n).toBe(192);
      await data.client.query("DROP TRIGGER fail_rating ON ratings");
      await data.client.query("DROP FUNCTION reject_tournament_rating()");
      await data.cleanup();
    } finally { await data.close(); }
  });

  test("targeted teardown protects foreign participants and removes dependent records", async ({ databaseUrl }, testInfo) => {
    const data = new TournamentData(databaseUrl, testInfo.outputPath("owner.json"));
    await data.open();
    try {
      const [owner, member] = await data.accounts(2);
      const target = await data.seed(owner, { players: [member] });
      const foreign = "00000000-0000-4000-8000-000000000002";
      data.manifest.accounts[0] = { id: foreign, username: "user_1", admin: false };
      await expect(data.cleanup()).rejects.toThrow("Invalid tournament ownership manifest");
      expect((await data.client.query("SELECT id FROM users WHERE id=$1", [foreign])).rowCount).toBe(1);
      data.manifest.accounts[0] = owner;
      await data.client.query("INSERT INTO tournaments_invitations (tournament_id,invitee_id,created_at) VALUES ($1,$2,now())", [target.id, foreign]);
      await expect(data.cleanup()).rejects.toThrow("Foreign participant or game");
      expect((await data.client.query("SELECT id FROM tournaments WHERE id=$1", [target.id])).rowCount).toBe(1);
      expect((await data.client.query("SELECT id FROM users WHERE id=$1", [owner.id])).rowCount).toBe(1);
      await data.client.query("DELETE FROM tournaments_invitations WHERE tournament_id=$1", [target.id]);
      await data.client.query("INSERT INTO tournaments_invitations (tournament_id,invitee_id,created_at) VALUES ($1,$2,now())", [target.id, owner.id]);
      // Exercise the real cascade graph, including games.tournament_id's missing FK.
      const game = (await data.client.query(`INSERT INTO games
        (nanoid,current_player_id,white_id,black_id,finished,game_status,game_type,history,game_control_history,
          rated,tournament_queen_rule,turn,created_at,updated_at,time_mode,time_base,time_increment,speed,hashes,
          conclusion,tournament_id,tournament_game_result,game_start,move_times)
        VALUES ('cleanupGame',$1,$1,$2,false,'NotStarted','MLP','','',true,true,0,now(),now(),
          'Real Time',600,10,'Rapid','{}','Unknown',$3,'Unknown','Ready','{}') RETURNING id`, [owner.id, member.id, target.id])).rows[0];
      await data.client.query("INSERT INTO schedules (game_id,tournament_id,proposer_id,opponent_id,start_t) VALUES ($1,$2,$3,$4,now())", [game.id, target.id, owner.id, member.id]);
      const channel = (await data.client.query("INSERT INTO chat_channels (kind,tournament_id) VALUES ('tournament_lobby',$1) RETURNING id", [target.id])).rows[0];
      await data.client.query("INSERT INTO chat_messages (channel_id,sender_id,body,client_id) VALUES ($1,$2,'Fixture cleanup message',gen_random_uuid())", [channel.id, owner.id]);
      await data.cleanup();
      await data.cleanup(); // Idempotent recovery after completed teardown.
      expect((await data.client.query("SELECT count(*)::int AS n FROM users")).rows[0].n).toBe(32);
      expect((await data.client.query("SELECT count(*)::int AS n FROM tournaments")).rows[0].n).toBe(0);
      expect((await data.client.query("SELECT count(*)::int AS n FROM tournaments_invitations")).rows[0].n).toBe(0);
      for (const table of ["games", "schedules", "chat_channels", "chat_messages"]) {
        expect((await data.client.query(`SELECT count(*)::int AS n FROM ${table}`)).rows[0].n).toBe(0);
      }
    } finally { await data.close(); }
  });

  test("recovery refuses an active owner and recovers after its database session is killed", async ({ databaseUrl }, testInfo) => {
    const manifest = testInfo.outputPath("owner.json");
    const data = new TournamentData(databaseUrl, manifest);
    const killer = new Client({ connectionString: databaseUrl });
    const recoverFromCLI = () => promisify(execFile)(process.execPath,
      [path.resolve(__dirname, "../support/tournaments/recover.mjs"), manifest],
      { env: { ...process.env, PLAYWRIGHT_DATABASE_URL: databaseUrl }, timeout: 10_000 });
    await data.open();
    await killer.connect();
    try {
      const [owner] = await data.accounts(1);
      const target = await data.seed(owner);
      await expect(recoverFromCLI()).rejects.toThrow("active owner");
      // Model death between UI creation and saving the returned ID to the manifest.
      await transaction(data.client, async () => {
        await data.client.query("DELETE FROM tournaments WHERE id=$1", [target.id]);
        await data.client.query(`INSERT INTO tournaments
          (nanoid,name,description,scoring,tiebreaker,seats,min_seats,rounds,invite_only,mode,time_mode,start_mode,status,created_at,updated_at)
          VALUES ('recoveredUI',$1,'Owned UI-created tournament','Game',ARRAY['RawPoints'],2,2,1,false,'DoubleRoundRobin','Real Time','Manual','NotStarted',now(),now())`, [target.name]);
        await data.client.query("INSERT INTO tournaments_organizers SELECT id,$2 FROM tournaments WHERE name=$1", [target.name, owner.id]);
      });
      const pid = (await data.client.query("SELECT pg_backend_pid() AS pid")).rows[0].pid;
      let lost = false;
      data.onLost = () => { lost = true; };
      await killer.query("SELECT pg_terminate_backend($1)", [pid]);
      await expect.poll(() => lost).toBe(true);
      expect(() => data.assertHeld()).toThrow("ownership was lost");
      await recoverFromCLI();
      expect((await killer.query("SELECT count(*)::int AS n FROM users")).rows[0].n).toBe(32);
      expect((await killer.query("SELECT count(*)::int AS n FROM tournaments")).rows[0].n).toBe(0);
    } finally { await data.close(); await killer.end(); }
  });
});
