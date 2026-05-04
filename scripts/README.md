# Blue-Green Deployment

Two systemd-managed instances (`hive@blue` on port 3000, `hive@green` on port 3001) sit behind nginx. Deploys flip between them with no HTTP downtime; WebSocket clients reconnect automatically (~1–3s spinner) and resume from DB state.

## Files in this directory

| File                              | Purpose                                                                 |
| --------------------------------- | ----------------------------------------------------------------------- |
| `deploy.sh`                       | Day-to-day deploy script (runs on the server).                          |
| `rollback.sh`                     | Brings the previous slot back online; refuses if migrations were introduced. |
| `systemd/hive@.service`           | Template service. Installs to `/etc/systemd/system/hive@.service`.      |
| `systemd/common.env`              | Shared `LEPTOS_*` env. Installs to `/etc/hive/common.env`.              |
| `systemd/blue.env`                | Blue port + per-slot site root. Installs to `/etc/hive/blue.env`.       |
| `systemd/green.env`               | Green port + per-slot site root. Installs to `/etc/hive/green.env`.    |
| `sudoers/hive-deploy`             | Lets `drone` run the systemctl/nginx commands without a password.       |
| `nginx/hive.conf.snippet`         | Reference snippet — splice into `/etc/nginx/sites-enabled/default`.     |

## Day-to-day deploy

```bash
/home/drone/hive/scripts/deploy.sh
```

The script:

1. **Detects the active slot from nginx** (`/etc/nginx/conf.d/hive-upstream-server.conf`), not from systemd. If nginx points at a color whose service is not running, it aborts — no auto-recovery.
2. Pulls latest, refreshes `.env` from `/home/leex/hive/.env`.
3. **Lints new migrations** for non-backward-compatible patterns (`DROP COLUMN`, `RENAME`, `SET NOT NULL`, etc.). Aborts unless `ALLOW_DESTRUCTIVE_MIGRATION=1` is set. See "Migration safety rule" below.
4. Builds with `cargo leptos build -rP`.
5. **Stages a per-slot release**: copies the binary to `bin/hive-<color>` AND the asset bundle to `bin/hive-<color>-site/`. The live slot's `LEPTOS_SITE_ROOT` points at *its own* dir, so a deploy never overwrites the running slot's hashed assets.
6. Starts the idle slot, waits for `/health` (liveness), then `/health/ready` (DB ping + asset presence), then `/` (full SSR roundtrip). Only then does it flip nginx.
7. Drains the old slot for 10s and stops it.

No separate `diesel migration run` step is needed — `apis/src/main.rs` calls `conn.run_pending_migrations(MIGRATIONS)` at startup, so the new instance migrates as it boots.

Logs:
```bash
sudo journalctl -u hive@blue -f
sudo journalctl -u hive@green -f
```

## Rollback

If the new version misbehaves, run:
```bash
/home/drone/hive/scripts/rollback.sh
```

This brings the previous slot back online and points nginx at it. The script:
- Detects the active color the same way `deploy.sh` does (from nginx, not systemd).
- Reads the recorded git SHA for each color from `bin/hive-{blue,green}.sha`.
- Checks `git diff <target_sha>..<active_sha> -- db/migrations/`. If anything shows up, the migrations have already been applied to the shared DB and rolling back risks corruption — the script refuses.
- Pass `--force` to override (only if you understand the consequences).

Rollback is safe with respect to assets: each slot has its own `bin/hive-<color>-site/` dir which is never touched while that slot is idle, so flipping back picks up a self-consistent (binary, assets) pair.

Limit: rollback only works **one step back**. After two deploys, the original binary has been overwritten by the third. For deeper rollback: `git checkout <old-sha>` and re-deploy.

## Migration safety rule

Migrations run automatically at app startup (`apis/src/main.rs`). During a deploy, the new instance migrates while the old keeps serving against the now-new schema. **All migrations must be backward-compatible** — additive only, no drops/renames, no `NOT NULL` without a default. Two-phase migrations span two deploys (e.g. add new column → switch readers/writers → drop old column).

`deploy.sh` greps each new `up.sql` for the obvious destructive patterns (`DROP COLUMN/TABLE/INDEX/CONSTRAINT/TYPE/SEQUENCE`, `ALTER ... RENAME/TYPE/SET NOT NULL`, `TRUNCATE`) and aborts unless `ALLOW_DESTRUCTIVE_MIGRATION=1`. The lint is intentionally a coarse safety net — it does not catch lock-heavy `ALTER`s, schema-incompatible defaults, or app-level invariants. Reviewer is still the source of truth.

## Singleton background jobs

`tournament_start`, `game_cleanup`, and `challenge_cleanup` would race during the blue/green overlap window if both slots ran them. Each one wraps its tick in a Postgres transaction-scoped advisory lock (`pg_try_advisory_xact_lock`) — only one slot wins per tick, and the lock auto-releases at commit. `heartbeat`/`ping` are intentionally not locked: they operate on each slot's own `WsServer` actor and only the nginx-routed slot's messages can reach connected clients.

## Initial bootstrap (one-time)

This is the cutover from the current "manual process on port 3000" to the blue-green setup.

```bash
cd /home/drone/hive
git pull
cp /home/leex/hive/.env .   # if your .env source-of-truth lives elsewhere
LEPTOS_HASH_FILES=true cargo leptos build -rP

# 1. Confirm port 3001 is free
sudo ss -tlnp | grep ':3001' && echo "WARNING: 3001 in use" || echo "3001 free"

# 2. Install systemd unit + env files
sudo install -m 644 scripts/systemd/hive@.service /etc/systemd/system/
sudo mkdir -p /etc/hive
sudo install -m 644 scripts/systemd/common.env /etc/hive/common.env
sudo install -m 644 scripts/systemd/blue.env /etc/hive/blue.env
sudo install -m 644 scripts/systemd/green.env /etc/hive/green.env

# 3. Install sudoers rule (visudo will reject it if syntax is wrong)
sudo install -m 440 scripts/sudoers/hive-deploy /etc/sudoers.d/hive-deploy

# 4. Stop the currently running manual process
sudo kill "$(pgrep -f '.cargo/target/release/apis')"

# 5. Stage initial binary AND assets into blue slot, then start
mkdir -p /home/drone/hive/bin
cp /home/drone/hive/.cargo/target/release/apis /home/drone/hive/bin/hive-blue
chmod +x /home/drone/hive/bin/hive-blue
rm -rf /home/drone/hive/bin/hive-blue-site
cp -a /home/drone/hive/target/site /home/drone/hive/bin/hive-blue-site
sudo systemctl daemon-reload
sudo systemctl enable --now hive@blue
curl -sf http://127.0.0.1:3000/health && echo "blue liveness OK"
curl -sf http://127.0.0.1:3000/health/ready && echo "blue readiness OK"

# 6. Update nginx — back up first, then splice per scripts/nginx/hive.conf.snippet
sudo cp /etc/nginx/sites-enabled/default /etc/nginx/sites-enabled/default.bak
sudo "$EDITOR" /etc/nginx/sites-enabled/default

# 7. Initial upstream-include file
echo "server 127.0.0.1:3000;" | sudo tee /etc/nginx/conf.d/hive-upstream-server.conf

# 8. Reload nginx
sudo nginx -t && sudo systemctl reload nginx

# 9. Verify both domains
curl -sf https://hivegame.com/health && echo "hivegame.com OK"
curl -sf https://hive.leex.dev/health && echo "hive.leex.dev OK"
```

After this, future deploys are just `./scripts/deploy.sh`.

## Verification of the swap mechanism

Run a no-op deploy (same code, just to flip slots):
```bash
./scripts/deploy.sh
```

In another terminal, hammer the health endpoint during the deploy:
```bash
while true; do curl -sf -o /dev/null -w "%{http_code}\n" https://hivegame.com/health; sleep 0.2; done
```
All responses should be `200`. Open a game in a browser; expect a brief "Connecting..." spinner around the cutover, then the game resumes (state loaded from DB).

To verify per-slot asset isolation specifically: during a deploy, hit a hashed asset on the *old* slot directly:
```bash
HASH_BUNDLE=$(curl -s https://hivegame.com/ | grep -oE '/pkg/HiveGame-[^"]+\.js' | head -1)
while true; do curl -sf -o /dev/null -w "%{http_code} $HASH_BUNDLE\n" "https://hivegame.com$HASH_BUNDLE"; sleep 0.2; done
```
Should stay `200` throughout — even while the new slot is being built.
