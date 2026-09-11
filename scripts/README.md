# Blue-Green Deployment

Two systemd-managed instances (`hive@blue` on port 3000, `hive@green` on port 3001) sit behind nginx. Deploys flip between them with no HTTP downtime; WebSocket clients reconnect automatically (~1–3s spinner) and resume from DB state.

## Files in this directory

| File                              | Purpose                                                                     |
| --------------------------------- | --------------------------------------------------------------------------- |
| `deploy.sh`                       | Day-to-day deploy script (runs on the server).                              |
| `rollback.sh`                     | Brings the previous slot back online; refuses if migrations were introduced. |
| `lib.sh`                          | Shared slot detection, health verification and nginx swap. Sourced by both.  |
| `systemd/hive@.service`           | Template service. Installs to `/etc/systemd/system/hive@.service`.          |
| `systemd/common.env`              | Shared `LEPTOS_*` env. Installs to `/etc/hive/common.env`.                  |
| `systemd/blue.env`                | Blue port, site root, telemetry path. Installs to `/etc/hive/blue.env`.     |
| `systemd/green.env`               | Same for green. Installs to `/etc/hive/green.env`.                          |
| `sudoers/hive-deploy`             | Lets `drone` run the systemctl/nginx commands without a password.           |
| `nginx/hive.conf.snippet`         | Reference snippet — splice into `/etc/nginx/sites-enabled/default`.         |

## Day-to-day deploy

```bash
/home/drone/hive/scripts/deploy.sh
```

The script:

1. Takes an exclusive `flock` on `.deploy.lock`, so a second deploy or rollback refuses rather than interleaving with this one.
2. **Detects the active slot from nginx** (`/etc/nginx/hive-upstream.conf`), not from systemd. If nginx points at a colour whose service is not running, it aborts — no auto-recovery.
3. `git pull --ff-only`.
4. **Dumps the database** to `db-backups/<timestamp>.dump`, keeping the newest `BACKUP_KEEP` (default 14) so the disk does not fill, in the background so it overlaps the build. The deploy aborts if the dump fails; `SKIP_BACKUP=1` opts out. The dump is waited on before the idle slot boots, because booting is what applies the migrations.
5. **Lints new migrations** for non-backward-compatible patterns (`DROP COLUMN`, `RENAME`, `SET NOT NULL`, etc.). Aborts unless `ALLOW_DESTRUCTIVE_MIGRATION=1` is set. See "Migration safety rule" below.
6. Builds with `nice -n 10 cargo leptos build -rP` — the live slot is on the same four physical cores.
7. **Stages a per-slot release** into `bin/<colour>/` — `hive` (the binary), `hash.txt`, `site/`, `sha`, `pkg.manifest`. Built in `bin/<colour>.new` and moved into place, then carries the outgoing release's hashed assets across. See "Slot layout" below.
8. Starts the idle slot, waits for `/health` (liveness), checks the slot is **bound to loopback only**, then `/health/ready` (DB ping + asset presence), then `/` (full SSR roundtrip). Only then does it flip nginx — validating with `nginx -t` first and restoring the old upstream file if nginx rejects it.
9. Drains the old slot for 3s and stops it.

Any failure **before** the flip stops the slot it just brought up, so a failed deploy does not leave two instances running indefinitely. **After** the flip it deliberately stops nothing — by then the new slot is the one serving traffic, and stopping it would turn a failed deploy into an outage. Either way it reports which slot nginx ended up on.

There is deliberately no `diesel migration run` step, unlike the old deploy script. `apis/src/main.rs` calls `conn.run_pending_migrations(MIGRATIONS)` at startup, so the new slot migrates as it boots — and *only* then. Running the CLI up front instead would apply the schema change before the build even starts, stretching the window where the old binary serves against the new schema from ~30s to the length of a full release build.

Logs:
```bash
sudo journalctl -u hive@blue -f
sudo journalctl -u hive@green -f
```

## Production secrets

The repo's `.env` is a **tracked** file holding dev/localhost values. Production secrets go in `/etc/hive/prod.env` (root-owned, `0640`, group `drone`), which `hive@.service` loads. Nothing in the deploy touches the repo's `.env`.

This is not just tidiness: writing prod secrets over the tracked file leaves the server's working tree permanently dirty, so `git pull` aborts the first time an upstream commit edits `.env` — and one `git add -A` on the server publishes the secrets.

## Rollback

If the new version misbehaves:
```bash
/home/drone/hive/scripts/rollback.sh
```

- Detects the active colour the same way `deploy.sh` does (from nginx, not systemd).
- Reads the recorded git SHA for each colour from `bin/{blue,green}/sha`.
- Checks `git diff <target_sha>..<active_sha> -- db/migrations/`. Anything found means the migrations are already applied to the shared DB and rolling back risks corruption — the script refuses. A `git diff` that *fails* (SHA gone, shallow clone) is treated the same way, because it is equally uninformative.
- Pass `--force` to override.

Rollback is safe with respect to assets: each slot owns its whole `bin/<colour>/` tree, untouched while that slot is idle, so flipping back picks up a self-consistent (binary, hash manifest, assets) set. It carries assets across in the same direction a deploy does.

Limit: rollback only works **one step back**. After two deploys, the original binary has been overwritten by the third. For deeper rollback: `git checkout <old-sha>` and re-deploy.

## Slot layout

```
bin/blue/
  hive           the release binary
  hash.txt       the asset hash manifest
  site/          LEPTOS_SITE_ROOT for this slot
  sha            the commit this release was built from
  pkg.manifest   this release's own asset filenames
```

One directory per slot, and the binary lives **inside** it, because leptos
resolves the hash manifest as `current_exe().parent()/hash.txt`. Two binaries
sharing a parent would read one manifest between them, and each deploy would
overwrite the running slot's copy — which with `LEPTOS_HASH_FILES=true` is not a
degraded mode but a panic on render (`expect("failed to read hash file")`), or
silently wrong `/pkg/` URLs. `deploy.sh` fails if the build produced no
`hash.txt` rather than staging a slot that 500s on every page.

### Assets across the flip

A page served by the outgoing slot fetches its hashed bundle *after* the flip,
by which time nginx points at the incoming slot. So staging carries the outgoing
release's `pkg` files across first, driven by that slot's `pkg.manifest` — one
generation, not the accumulated union of every release that ever ran.

## Migration safety rule

Migrations run automatically at app startup. During a deploy, the new instance migrates while the old keeps serving against the now-new schema. **All migrations must be backward-compatible** — additive only, no drops/renames, no `NOT NULL` without a default. Two-phase migrations span two deploys (add new column → switch readers/writers → drop old column).

`deploy.sh` greps each new `up.sql` for the obvious destructive patterns and aborts unless `ALLOW_DESTRUCTIVE_MIGRATION=1`. The lint is a coarse safety net — it does not catch lock-heavy `ALTER`s, schema-incompatible defaults, or app-level invariants. Reviewer is still the source of truth.

## Background jobs and the overlap window

Both slots run every job for the length of the overlap — from the moment the idle slot boots until the old one is stopped, roughly 20–70s. Which jobs need protecting is not "all of them":

**Protected, because a second run does real work twice:**

- `tournament_start`, `game_cleanup`, `challenge_cleanup` — `pg_try_advisory_xact_lock` around the tick; the loser skips and the lock auto-releases at commit.
- `hash_backfill` — `pg_try_advisory_lock` (session-scoped, held on a dedicated connection for the whole pass, since it spans many transactions). Two instances would replay the same games from the same cursor, and during a rehash they would be running *different* hash algorithms.
- `email_drain` — not a lock. `EmailQueueItem::claim_batch` now leases its batch by pushing `scheduled_at` forward in the same statement that selects it (`FOR UPDATE SKIP LOCKED`), so the other instance's `scheduled_at <= now()` filter no longer matches. Before this, both instances read the same unsent rows and both delivered — password resets went out twice on every deploy.

**Deliberately not locked:**

- `heartbeat`, `ping` — operate on each slot's own `WsHub`; only the nginx-routed slot's messages reach clients.
- `ws_telemetry` — each slot writes its own CSV (`WS_METRICS_LOG_FILE` in `blue.env`/`green.env`). Sharing the default `./ws_metrics.csv` interleaved two processes' rows into one file.
- `timeout_sweeper` — already idempotent (`check_time` returns the row unchanged when a move reset the clock, and the broadcast is gated on the conclusion actually being `Timeout`). Locking it would be a downgrade: it fires every 60s and notifies clients, so the loser skipping means the slot users are *actually connected to* never sends the timeout.
- `tournament_cleanup`, `push_device_sweep`, `email_cleanup` — daily, threshold-based deletes that return nothing on a second pass. `tournament_cleanup` also broadcasts, so a lock would carry the same downside as `timeout_sweeper` for no gain.

The general shape: lock a job when running it twice is *harmful*, not merely wasteful — and prefer making the work idempotent over locking when the job also notifies WebSocket clients, because the lock winner may be the slot with no clients on it.

Known consequence, accepted: `tournament_start` is locked *and* notifies. If the idle slot wins that tick during an overlap, the start notification goes to a hub with no clients and connected users see the tournament on their next navigation rather than live. It fires every 60s, so this lands on a minority of deploys.

## The drain window

After the flip, sockets opened before it are still on the old slot while
everything new is on the new one. The two hubs cannot see each other's events,
so a move made by a player on one is not pushed to an opponent on the other.
Clients on the old slot are healed by the reconnect when it stops — they reload
state from the DB — but a client that connected to the *new* slot during the
window never reconnects, and stays stale until it next navigates.

The window is the drain plus however long the old slot takes to die, and that
second term used to dominate. Actix waits `shutdown_timeout` for connections to
close, a WebSocket never closes on its own, and the default is 30s — so even a
3s drain meant a measured 33s window. `main.rs` now sets `.shutdown_timeout(5)`,
and the drain is 3s rather than 10 because it only has to outlast in-flight SSR
requests, which are sub-second. Together, roughly 8s. Shrinking it further does
not remove it.

The real fix is to stop treating a hub as the whole world — have the slot that
raises an event publish it over Postgres `LISTEN`/`NOTIFY` so both hubs
broadcast to their own clients. That also removes the caveat on
`tournament_start` above, and would let the locked jobs notify correctly. It is
the obvious next piece of work here.

## Reboot

`deploy.sh` does not manage `systemctl enable`/`disable`, so a reboot starts whichever slot was enabled at bootstrap (blue). If the last deploy left **green** active, nginx's upstream still says `3001` and nothing is listening — 502 until you run `deploy.sh` (or `systemctl start hive@green`) by hand. Reboots are rare and supervised; check which colour is live before rebooting.

## Initial bootstrap (one-time)

Cutover from the current "manual process on port 3000" to blue-green.

```bash
cd /home/drone/hive
git pull --ff-only
LEPTOS_HASH_FILES=true cargo leptos build -rP

# 1. Confirm port 3001 is free
sudo ss -tlnp | grep ':3001' && echo "WARNING: 3001 in use" || echo "3001 free"

# 2. Install systemd unit + env files
sudo install -m 644 scripts/systemd/hive@.service /etc/systemd/system/
sudo mkdir -p /etc/hive
sudo install -m 644 scripts/systemd/common.env /etc/hive/common.env
sudo install -m 644 scripts/systemd/blue.env /etc/hive/blue.env
sudo install -m 644 scripts/systemd/green.env /etc/hive/green.env

# 3. Production secrets, outside the repo. Copy from wherever they live now
#    (the old manual process read /home/drone/hive/.env).
sudo install -m 640 -o root -g drone /path/to/prod.env /etc/hive/prod.env

# 4. Install sudoers rule (visudo will reject it if syntax is wrong)
sudo install -m 440 scripts/sudoers/hive-deploy /etc/sudoers.d/hive-deploy

# 5. Stop the currently running manual process
sudo kill "$(pgrep -f '.cargo/target/release/apis')"

# 6. Stage the blue slot. The hash manifest MUST land beside the binary —
#    find where this cargo-leptos writes it before assuming:
find /home/drone/hive -name hash.txt -newermt '-30 minutes' -not -path '*/node_modules/*'

BLUE=/home/drone/hive/bin/blue
mkdir -p "$BLUE"
cp /home/drone/hive/.cargo/target/release/apis "$BLUE/hive"
chmod +x "$BLUE/hive"
cp <the hash.txt found above> "$BLUE/hash.txt"
cp -a /home/drone/hive/target/site "$BLUE/site"
git rev-parse HEAD > "$BLUE/sha"
( cd "$BLUE/site/pkg" && find . -type f | sort ) > "$BLUE/pkg.manifest"
sudo systemctl daemon-reload
sudo systemctl enable --now hive@blue
curl -sf http://127.0.0.1:3000/health && echo "blue liveness OK"
curl -s http://127.0.0.1:3000/health/ready; echo

# 7. Update nginx — back up first, then apply scripts/nginx/hive.conf.snippet.
#    hivegame.com's `proxy_pass http://localhost:3000` and hive.leex.dev's
#    `try_files $uri $uri/ =404;` both become the same pair of locations; both
#    domains already have their own Certbot certificate.
# NOT inside sites-enabled/ — Debian includes that directory as `*`, not
#    `*.conf`, so a .bak there is loaded too and nginx -t fails with
#    "duplicate default server".
sudo cp /etc/nginx/sites-enabled/default /etc/nginx/default.bak
sudo "$EDITOR" /etc/nginx/sites-enabled/default

# 8. Initial upstream-include file
echo "server 127.0.0.1:3000;" | sudo tee /etc/nginx/hive-upstream.conf

# 9. Reload nginx
sudo nginx -t && sudo systemctl reload nginx

# 10. Verify both domains
curl -sf https://hivegame.com/health && echo "hivegame.com OK"
curl -sf https://hive.leex.dev/health && echo "hive.leex.dev OK"
```

The slots bind `127.0.0.1`, not `0.0.0.0`. This is what closes the exposure the pre-blue-green setup had: `cargo leptos serve` took its address from `apis/Cargo.toml` (`site-addr = "0.0.0.0:3000"`), and with no firewall on the box the app answered on the public IP in cleartext, around nginx and its TLS.

`Cargo.toml` is deliberately left at `0.0.0.0` — that is what a dev wants when testing from a phone on the LAN. Production gets the loopback address from `/etc/hive/<colour>.env`, and `verify_slot` refuses to flip nginx to a slot that is bound anywhere else, so a missing or stale env file fails the deploy instead of quietly re-opening the port.

After this, future deploys are just `./scripts/deploy.sh`.

## On-box API clients

`hive-hydra.prod.yaml` sets `base_url: "http://localhost:3000"` — the blue slot,
not "the app". After a deploy flips to green the bots keep dialling 3000, which
the drain has just stopped, and they stay dark until some later deploy happens
to flip back. Anything else on the box pointed at a slot port has the same
problem.

nginx listens on `127.0.0.1:3999` and proxies to whichever slot is live. Point
those clients there:

```yaml
base_url: "http://localhost:3999"
```

## hive.leex.dev

Serves the same app as hivegame.com, off the same active slot. The app's own
absolute URLs are hardcoded to `hivegame.com` — OG tags, password-reset links,
push-notification deep links, the PGN `Site` tag, the JWT issuer — so a session
started on hive.leex.dev works, but every link it hands back points at the
primary domain. Fine for a second front; it is not an independent deployment.

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

To verify the asset carryover, grab the bundle URL of the *outgoing* release
before starting a deploy and hold it across the flip:
```bash
HASH_BUNDLE=$(curl -s https://hivegame.com/ | grep -oE '/pkg/HiveGame[^"]+\.js' | head -1)
while true; do curl -sf -o /dev/null -w "%{http_code} $HASH_BUNDLE\n" "https://hivegame.com$HASH_BUNDLE"; sleep 0.2; done
```
This is the check that matters, and it is the one that fails without the
carryover: staying `200` *during the build* proves nothing, since the old slot
is still serving. It has to stay `200` **after** the flip, when nginx is
pointing at a slot that never built that file.

And confirm the hash manifest actually shipped, since a missing one is a render
panic rather than a degraded page:
```bash
ls -l /home/drone/hive/bin/{blue,green}/hash.txt
```
