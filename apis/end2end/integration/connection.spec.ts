import { Client } from "pg";
import { test, expect } from "./database";
import { reserveAccounts } from "../support/accounts/pool";

test.describe("Connection loss", () => {
  test("losing a database session notifies the fixture and invalidates the reservation", async ({ databaseUrl }) => {
    const reservation = await reserveAccounts(2, { connectionString: databaseUrl });
    const db = new Client({ connectionString: databaseUrl });
    await db.connect();
    let lost: Error | undefined;
    const unsubscribe = reservation.onLost(error => { lost = error; });
    try {
      await db.query(`SELECT pg_terminate_backend(pid) FROM pg_stat_activity
        WHERE datname = current_database() AND application_name = $1`, [`hive-playwright-${process.pid}`]);
      await expect.poll(() => lost?.message).toContain("Lost the database connection");
      expect(() => reservation.assertHeld()).toThrow("Lost the database connection");
      await expect(reservation.state()).rejects.toThrow("Lost the database connection");
    } finally {
      unsubscribe();
      await reservation.release();
      await db.end();
    }
  });

  test("connection failures do not disclose credentials", async () => {
    await expect(reserveAccounts(2, { connectionString: "postgres://secret_user:secret_password@127.0.0.1:1/missing" }))
      .rejects.toThrow(/^Cannot connect to PLAYWRIGHT_DATABASE_URL; check test database access\.$/);
  });
});
