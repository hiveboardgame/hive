# Blue-Green Deployment

Two systemd instances, `hive@blue` (127.0.0.1:3000) and `hive@green` (127.0.0.1:3001), sit
behind nginx. Each runs an immutable release from `releases/<sha>/`; `bin/blue` and
`bin/green` are symlinks to the release they run. A deploy builds a new release, migrates,
starts it on the idle slot, verifies it, flips nginx, then stops the old slot.

## Daily use

As `drone` (`sudo su - drone`), in `/home/drone/hive`:

```bash
scripts/deploy.sh     # ship the branch; ends with "OK: <slot> is live ... smoke passed"
scripts/rollback.sh   # back to the previous release (from the second deploy on)
scripts/status.sh     # what is live right now
```

Run it inside tmux (a dropped SSH session kills a deploy midway). You never pick blue or
green; the scripts do. Only one deploy or rollback runs at a time.
If a script refuses or ends in FAIL, paste its output (and `scripts/status.sh`) in chat
before touching anything. Every migration must keep the previous release working.

| File                                  | Purpose / installs to                              |
| ------------------------------------- | -------------------------------------------------- |
| `deploy.sh`, `rollback.sh`, `lib.sh`  | deploys, run from the checkout as `drone`          |
| `status.sh`, `preflight.sh`, `smoke.sh` | read-only checks, paste their output when asking for help |
| `run-hydra.sh`, `run-busybee.sh`, `run-evaluator.sh`, `tmux-session.sh` | on-box services in drone's tmux |
| `systemd/hive@.service`               | `/etc/systemd/system/hive@.service`                |
| `systemd/{common,blue,green}.env`     | `/etc/hive/`                                       |
| `sudoers/hive-deploy`                 | `/etc/sudoers.d/hive-deploy`                       |
| `sbin/hive-upstream`                  | `/usr/local/sbin/hive-upstream` (root 0755): the only way `drone` switches nginx |
| `nginx/sites-enabled-default.example` | `/etc/nginx/sites-available/default`               |

Secrets: the app reads `/etc/hive/prod.env` (root:drone, 0640). Scripts reach the database
through libpq service `hive` (`~/.pg_service.conf` + `~/.pgpass`). Rotating the database
password means updating both.

## Commands

```bash
scripts/status.sh             # what nginx, systemd and the releases say right now
scripts/preflight.sh          # every prerequisite, PASS/WARN/FAIL; read-only
scripts/smoke.sh              # the live site checked from outside; read-only
scripts/deploy.sh             # deploy HEAD of the checkout's branch
scripts/deploy.sh --build-only  # pull, build + stage the release; touches no running process
scripts/deploy.sh --bootstrap   # once: from the hand-started process (see "First cutover")
scripts/rollback.sh [--force] # go back to the previous successful release
```

## Deploy

1. Takes `flock` on `.deploy.lock`, then refuses unless the state is consistent: nginx
   names a slot, that slot runs and is enabled for boot, the other slot neither runs nor is
   enabled, `bin/<live>` points at `releases/current`, and `releases/active` names it.
2. `git pull --ff-only` (skipped on a detached HEAD).
3. Reads the applied migrations from the database and checks that both the live release and
   the new one can run against them plus the new release's migrations: every migration a
   release does not know must look additive. Otherwise it stops unless
   `ALLOW_DESTRUCTIVE_MIGRATION=1`. This is a grep, not a proof; see the rules below.
4. `pg_dump` into `db-backups/` in the background (newest `BACKUP_KEEP`=14 kept;
   `SKIP_BACKUP=1` skips it).
5. Builds in a separate worktree, `/home/drone/hive-build`, so a build never touches what a
   running process serves. The commit is baked into the binary and returned by `/health`.
   A release that already exists is reused.
6. Waits for the dump, then runs `hive --migrate-only` from the new release with
   `lock_timeout=10s` and `statement_timeout=300s` (`MIGRATION_LOCK_TIMEOUT`,
   `MIGRATION_STATEMENT_TIMEOUT`): a migration stuck on a lock fails instead of stalling the site.
7. Points the idle slot at the release, starts it, checks `/health` (expected commit),
   loopback-only bind, `/health/ready`, `/`.
8. Flips nginx and checks the expected commit through nginx; if that fails, flips back.
9. Enables the new slot for boot, disables the old one, waits 1s, stops the old slot.
10. Writes `releases/active`: only now does the new slot run jobs and accept websockets.
11. Records `releases/current` and `releases/previous`, prunes old releases.
12. Runs `smoke.sh` and ends with one line: `OK: …` or `DEPLOYED, BUT SMOKE FAILED: …`.

On any failure or interrupt (Ctrl-C, dropped SSH), cleanup looks at where nginx routes and
makes everything agree: still the old side, it stops the new slot; the new slot but not yet
verified, it flips back and stops it; the verified new slot, it finishes the handoff (stops
the old side, boot slot, `active`, markers). A second Ctrl-C does not interrupt cleanup.
Run as `drone` only; the scripts refuse other users.

Logs: `journalctl -u hive@blue -u hive@green -f`.

## Rollback

Starts `releases/previous` on the idle slot and flips to it, with the same handoff as a
deploy. Refuses (unless `--force`) if any migration applied in the database that the previous
release does not know looks destructive, including migrations applied by a deploy that
failed later. A failed deploy never touches `previous`, so it stays a valid target.
Afterwards `deploy.sh` refuses to ship the rolled-back commit again: push a fix or a revert
first (or set `REDEPLOY_ROLLED_BACK=1`).

## Rules

- **Every migration must keep the previous release working.** Additive changes only; drops,
  renames and new `NOT NULL` columns take two deploys. Migrations run before traffic flips,
  so a deploy that fails after migrating has still changed the schema. Restoring a dump
  loses every write made since it was taken.
- **Browser and server must stay compatible for one release.** Pages and the service worker
  keep old JS/WASM running against the new server. Server-function arguments and WebSocket
  message types change additively, or the change ships in two releases.
- **One active instance at a time.** `releases/active` names the instance that runs
  background jobs and accepts websockets. The scripts move it only after nginx verifiably
  serves the new slot and the old slot has stopped, so the two never run jobs or hold
  players at the same time. As a backstop, `tournament_start`, `game_cleanup`,
  `challenge_cleanup` and `hash_backfill` also take advisory locks and `email_drain` leases its
  batch; the other jobs rely on `active` alone.
- **Players reconnect once at the flip.** Until the handoff, the new slot answers websocket
  upgrades with 503 and clients retry (2s, then 4s). Everyone ends up on the new slot after
  the old one has stopped, so every move made through the old slot is already in the
  database when they load state. Expect a 2–6s "connecting" spinner.
- nginx serves `/pkg/` from both slots, so a page rendered by the old release still finds
  its bundle after the flip (not at the bootstrap flip: a tab caught mid-load then needs
  a refresh).
- `hive-hydra` talks to `http://localhost:3999`, which follows the live slot.

## Recovering

`deploy.sh` and `rollback.sh` print the problems and `status.sh` output when they refuse.
Decide which slot should be live (normally the one nginx names and `/health` answers through
`via nginx`), then make the rest agree:

```bash
sudo systemctl stop hive@<other>                    # other slot still running
sudo systemctl enable hive@<live>                   # boot state wrong
sudo systemctl disable hive@<other>
echo <sha of bin/<live>> > /home/drone/hive/releases/current   # marker stale
echo <release live before it> > /home/drone/hive/releases/previous   # rollback target
echo 127.0.0.1:<live port> > /home/drone/hive/releases/active  # only once the other side is stopped
```

If nginx names a slot that is not running, start it (`sudo systemctl start hive@<colour>`)
or point nginx at the running one (`sudo /usr/local/sbin/hive-upstream <colour>`). Re-run `status.sh` until it is consistent.

A reboot in the middle of a deploy can leave the newly enabled slot running but not active
(`status.sh`: it runs, nginx names it, `active` names the other one): websockets get 503 and no
jobs run. If that slot answers `/health` with the release you meant to ship, make it active:
`echo 127.0.0.1:<its port> > /home/drone/hive/releases/active`, then fix the markers as above.

## First cutover (one-time)

Today the app runs by hand in drone's tmux on `0.0.0.0:3000`. The cutover first moves nginx
to the new config while it still routes to that old process, then `deploy.sh --bootstrap`
brings up **green**, flips to it, and stops the old process itself. Steps marked **(leex)**
need full sudo (`leex` or `ion`); everything else runs as **drone** (`sudo su - drone`), which
has no sudo beyond `sudoers/hive-deploy`. Every step ends with a checkpoint: paste the output.

### Before the window

```bash
# (drone) BEFORE pulling the merge: record the commit the running binary was built from.
cd /home/drone/hive
git rev-parse HEAD > ~/old-release.sha
git log -1 --format='%h %ci' ; stat -c '%y' .cargo/target/release/apis   # commit not newer than the binary
# the running process's real LEPTOS_* settings, reused to start it again by hand
chmod 600 .env                                              # prod secrets, until step 6
OLD_PID=$(pgrep -f '\.cargo/target/release/apis$'); echo "$OLD_PID"
(umask 077; cp /proc/$OLD_PID/environ ~/old-process.environ)   # its whole environment
tr '\0' '\n' < ~/old-process.environ | cut -d= -f1 | sort | tr '\n' ' '; echo   # names only
chmod 700 db-backups && chmod 600 db-backups/*.dump
git pull --ff-only

# (drone) database service for pg_dump, migrations and psql; busybee secrets
printf '[hive]\nhost=localhost\ndbname=hive-local\nuser=hive-dev\n' > ~/.pg_service.conf
echo 'localhost:5432:hive-local:hive-dev:<PASSWORD>' > ~/.pgpass && chmod 600 ~/.pgpass
mkdir -p ~/.config/busybee && chmod 700 ~/.config/busybee   # then the DISCORD_*/BUSYBEE_* lines, 0600

# (leex)
sudo usermod -aG systemd-journal drone

# (drone) warm the build (separate worktree; the running process is untouched), then check
scripts/deploy.sh --build-only
scripts/preflight.sh
```

**Checkpoint A.** PASS for tools, database access, the recorded old commit, nginx
mime/gzip_static, www-data access, disk, `release built`, pending migrations listed, schema
fit. FAIL only for `/etc/hive/*`, `common.env`, `systemd knows hive@.service`, the helper,
sudo and `nginx -t` (steps 2–3). WARN for the nginx site (step 3), hydra base URL (step 4),
systemd-journal (until the reboot), `tracked changes: .env`, `:3000` and `:8080 listens publicly`. Read the pending migrations'
SQL together.

### In the window

**1. Rotate the database password, then reboot.** (leex) Check open realtime games and
tournament starts, pause UptimeRobot, stop hydra and busybee. Then (drone), immediately before
the reboot (the running app keeps its open connections, but new ones need the new password):

```bash
cd /home/drone/hive
bash <<'ROTATE'
set -euo pipefail
URL=$(sed -nE 's/^DATABASE_URL="?([^"]*)"?$/\1/p' .env | head -1)
[[ $URL =~ ^postgres(ql)?://([^:@/]+):([^@]*)@([^:/]*)(:([0-9]*))?/(.+)$ ]] || { echo "DATABASE_URL did not parse"; exit 1; }
U=${BASH_REMATCH[2]} H=${BASH_REMATCH[4]} PORT=${BASH_REMATCH[6]:-5432} DB=${BASH_REMATCH[7]}
grep -q "^$H:$PORT:$DB:$U:" ~/.pgpass || { echo "no .pgpass line for $H:$PORT:$DB:$U"; exit 1; }
NEW=$(od -An -N24 -tx1 /dev/urandom | tr -d ' \n')
umask 077
echo "$NEW" > ~/.db-password.new                        # kept until step 6, in case a step below fails
cp .env ~/.env.before-rotation; cp ~/.pgpass ~/.pgpass.before-rotation
sed -E "s#^DATABASE_URL=.*#DATABASE_URL=\"postgres://$U:$NEW@$H:$PORT/$DB\"#" .env > .env.rotating
sed -E "s#^($H:$PORT:$DB:$U):.*#\1:$NEW#" ~/.pgpass > ~/.pgpass.rotating
printf "ALTER ROLE CURRENT_USER PASSWORD '%s';\n" "$NEW" | psql service=hive -X -q -v ON_ERROR_STOP=1
mv .env.rotating .env; mv ~/.pgpass.rotating ~/.pgpass
echo "new password works: $(psql service=hive -XAtc 'select current_user')"
ROTATE
ls -l .env ~/.pgpass
```

Expect `new password works: hive-dev` and both files `-rw-------`. If it stops before or at
`ALTER ROLE`, nothing changed for Postgres: delete the `*.rotating` files and re-run. If it
stops after `ALTER ROLE`, Postgres already has the new password, which is in
`~/.db-password.new`: run the two `mv` commands (or, if the `*.rotating` files are missing, put
that password into `.env` and the `.pgpass` line by hand). Never restore the `*.before-rotation`
files then; they hold the old password. Then (leex) `sudo reboot`, and check `uname -r` = `6.12.111+deb13-amd64`. (drone)
`scripts/tmux-session.sh`; in its `shell` window start the old binary (no rebuild, old code):

```bash
bash -c 'while IFS= read -r -d "" kv; do [[ $kv == DATABASE_URL=* ]] || export "$kv"; done < ~/old-process.environ; exec .cargo/target/release/apis'   # bash (drone's shell is zsh); DATABASE_URL comes from .env
```

**Checkpoint 1.** The site loads, the hydra/busybee/psql windows run without errors.

**2. System files.** (leex), from `/home/drone/hive`:

```bash
sudo install -d -m 755 /etc/hive
sudo install -m 640 -o root -g drone .env /etc/hive/prod.env
sudo install -m 644 scripts/systemd/hive@.service /etc/systemd/system/
sudo install -m 644 scripts/systemd/common.env scripts/systemd/blue.env scripts/systemd/green.env /etc/hive/
sudo install -m 755 -o root -g root scripts/sbin/hive-upstream /usr/local/sbin/hive-upstream
sudo systemctl daemon-reload
sudo visudo -cf scripts/sudoers/hive-deploy && \
  sudo install -m 440 scripts/sudoers/hive-deploy /etc/sudoers.d/hive-deploy
```

**Checkpoint 2.** (drone) `scripts/preflight.sh`: no FAIL. WARN only for the nginx site, the
hydra base URL, `tracked changes: .env` and `:3000`.

**3. nginx, still pointing at the old process.** (leex)

```bash
sudo cp /etc/nginx/sites-available/default /etc/nginx/default.bak
sudo install -m 644 scripts/nginx/sites-enabled-default.example /etc/nginx/sites-available/default   # diff reviewed 2026-10-06
echo "server 127.0.0.1:3000;" | sudo tee /etc/nginx/hive-upstream.conf
sudo nginx -t && sudo nginx -s reload
diff /etc/nginx/default.bak /etc/nginx/sites-available/default
```

**Checkpoint 3.** `nginx -t` OK; the diff shows only the blue-green changes; the site still
loads (bundles come from the old process through the `@app` fallback);
`curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:3999/` prints `200`.
Undo: `sudo cp /etc/nginx/default.bak /etc/nginx/sites-available/default && sudo rm
/etc/nginx/hive-upstream.conf && sudo nginx -t && sudo nginx -s reload`.

**4. hydra onto 3999.** (drone) Add `HIVE_HYDRA_BASE_URL=http://localhost:3999` to
`~/.config/hive-hydra/env`, Ctrl-C the hydra window, `scripts/run-hydra.sh`.

**Checkpoint 4.** Bots still play (3999 still routes to the old process).

**5. Bootstrap.** (drone)

```bash
LIVE_SHA=$(cat ~/old-release.sha) scripts/deploy.sh --bootstrap
```

It does not pull: it deploys the commit checkpoint A checked. It checks the schema against
both commits, backs up, migrates, starts green, flips nginx, verifies it through nginx
(flips back to the old process if not), stops the old process (SIGKILL after 3s), hands jobs
and websockets to green, and runs the smoke checks. Players reconnect within ~5–15s.

**Checkpoint 5.** The log (`deploy-logs/*-bootstrap.log`) ends with
`OK: green is live with release … smoke passed`.

**6. Retire the old setup.** (drone)

```bash
git checkout .env
rm deploy.sh psql.sh ~/.env.before-rotation ~/.pgpass.before-rotation ~/.db-password.new   # psql.sh: old, leaked password
(crontab -l 2>/dev/null; echo '@reboot /home/drone/hive/scripts/tmux-session.sh') | crontab -   # hydra + busybee after reboots
scripts/preflight.sh
```

**Checkpoint 6.** No FAIL. Then resume UptimeRobot with the `/health` monitor, and check by
hand: a password-reset mail arrives, a push notification arrives, Discord linking works, a
bot accepts a challenge.

### Backing out

Until step 6 the old setup is intact: the separate build never touched `.cargo/target` or
`target/site` in the checkout, and the tracked `.env` still holds the prod values. Do not run
`cargo leptos` in the checkout during the window. Before step 5: put hydra back (remove
`HIVE_HYDRA_BASE_URL` from its env, restart `scripts/run-hydra.sh`), then undo step 3 (above).
After step 5, as drone (after step 6, first `cp /etc/hive/prod.env .env && chmod 600 .env`):

```bash
rm -f releases/active releases/current releases/previous   # green stops jobs and new websockets
sudo systemctl stop hive@green && sudo systemctl disable hive@green   # returns once green has exited
# only now, in the tmux shell window: the step 1 command (the old process), then:
sudo /usr/local/sbin/hive-upstream blue                    # :3000 = the old process
```

The site answers 502 for the few seconds between stopping green and the flip. That is on
purpose: the old process is always active, so it must never run next to green.

Run the whole window inside tmux: a dropped SSH session kills a foreground `deploy.sh`.

## tmux

`scripts/tmux-session.sh` creates (or attaches to) session `hive`: `hive` (slot logs),
`hydra`, `busybee`, `evaluator`, `psql`, `shell`. A crashed window drops to a shell showing the error.
Needs `~/.config/hive-hydra/env`, `~/.config/busybee/env` and `~/.config/hive-evaluator/env`
(all 0600; DISCORD_*, BUSYBEE_*, see Evals below),
`busybee/venv`, the `hive` libpq service, and `drone` in group `systemd-journal`.
For a session after every reboot: `@reboot /home/drone/hive/scripts/tmux-session.sh` in
drone's crontab.

## Evals

Engine evals of finished games (the Evals tab in analysis, "Recent evaluations" on the front
page) come from `hive-evaluator`, which drives the StockBee engine; see
`hive-evaluator/README.md`. StockBee never goes into this repo: it lives in `~/stockbee`.

Until the 2026 world championship is over, evals are for admins only: everyone else sees no
Evals tab, no marks in History, no ghosts on the board and no front-page card, and the server
refuses them the eval data. The worker runs as normal behind that.

### Rolling out

1. Copy StockBee to the box without build output (from a machine that has it):
   `rsync -a --exclude build --exclude out --exclude venv stockbee/ drone@<box>:stockbee/`
2. As drone: `scripts/setup-evaluator.sh`. It locks the directory to drone, makes the eval
   server load the net as plain weights, builds the engine, creates the torch venv, and writes
   `~/.config/hive-evaluator/env` with a new token and `EVAL_AUTO=false`.
3. Once, as a user with sudo: `sudo scripts/setup-evaluator.sh --install-token` copies that
   token into `/etc/hive/prod.env` without printing it.
4. Keep the worker API off the internet. In `/etc/nginx/sites-available/default`, add to the
   `hivegame.com` server block, next to the `/health/ready` deny (see
   `nginx/sites-enabled-default.example`):

   ```nginx
   location /api/v1/evals/ {
       deny all;
   }
   ```

   then `sudo nginx -t && sudo systemctl reload nginx`. The worker is not affected: it reaches
   the site on `127.0.0.1:3999`. A worker elsewhere (a laptop, a rented GPU) can be let in
   later with `allow <its IP>;` above the deny; until then its requests get 403.
5. `scripts/preflight.sh`: it should show the evaluator token, env file and StockBee directory,
   and "site keeps the eval worker API off the internet".
6. Merge and `scripts/deploy.sh`, so the site has the eval code and reads the token.
7. As drone, `scripts/setup-evaluator.sh` again: it sees the matching token and opens the
   `evaluator` tmux window.
8. `scripts/smoke.sh`: "hive-evaluator running" and "the eval worker API is not public", and
   `curl -s -o /dev/null -w '%{http_code}\n' -X POST https://hivegame.com/api/v1/evals/claim`
   answers 403.
9. As an admin, open a finished MLP tournament game, Evals tab, request an eval, and watch it
   go from queued to running to done. Logged out (or as a non-admin) the same page has no
   Evals tab, and the front page no "Recent evaluations" card.

### Opening evals to everyone

After the world championship:

1. Delete `ensure_eval_access` and its three calls in `apis/src/functions/game_evals.rs`, and
   make `evals_visible` in `apis/src/providers/game_eval.rs` return true (or remove it and its
   uses in the analysis sidebar and on the home page).
2. Merge and deploy as usual; nothing changes for the worker.
3. Optional: set `EVAL_AUTO=true` in `~/.config/hive-evaluator/env` and restart its window, so
   the front page fills with evals of strong games while nobody is waiting.

### Running it

The worker takes user requests first. With `EVAL_AUTO=true` in its env (restart its window:
Ctrl-C, then `scripts/run-evaluator.sh`) it also evaluates strong games while nobody is
waiting. It talks to `:3999`, is built at nice 15 and runs at nice 19 with three eval servers
of two torch threads each, survives slot flips, and an eval it drops is requeued after five
quiet minutes. Stopping it (Ctrl-C) switches evals off without losing any; the site then says
no engine is running.

`scripts/preflight.sh` checks the token, the env file, the StockBee directory and the nginx
block (WARN while evals are not set up). Re-running `scripts/tmux-session.sh` on a live session
adds windows that are missing, so it also brings back a closed `evaluator` window.

## Checking a deploy

After every deploy or rollback, `scripts/smoke.sh` should end with `smoke: 0 failed`. To watch
a flip as it happens, run these in a second terminal during the deploy:

```bash
while true; do curl -s -m 2 https://hivegame.com/health; echo; sleep 0.2; done
```

The commit should change once, with no errors. To check old bundles survive the flip, grab
one before deploying and poll it until after:

```bash
B=$(curl -s https://hivegame.com/ | grep -oE '/pkg/HiveGame[^"]+\.wasm' | head -1)
while true; do curl -sf -o /dev/null -w "%{http_code} $B\n" "https://hivegame.com$B"; sleep 0.5; done
```
