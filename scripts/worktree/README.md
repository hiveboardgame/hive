# Worktrees: one branch, one database, one URL

Run several branches side by side, each in its own checkout with its own database, port
and hostname:

```sh
setup tournamint        # worktree + database + http://tournamint.localhost
run tournamint watch    # serve it
cleanup                 # pick what to remove
```

The commands come from the nix dev shell of your main checkout (`direnv` or
`nix develop`). They need nothing else installed: Caddy, gum and the Postgres 18 client
tools are pinned in `flake.nix`.

## How it fits together

- Worktrees live in a directory next to your main checkout: `~/src/hive` gets
  `~/src/hive-worktrees/<name>`. Set `HIVE_WORKTREES_DIR` to put them elsewhere. Each is a
  normal `git worktree` sharing the main checkout's repository, so you commit and push from
  inside it as usual.
- Each worktree gets a copy of `hive-template` as `hive-<name>` and an empty
  `hive-<name>-test` for `cargo test -p db`. The app migrates its own database on start,
  so branches with new migrations never touch each other or `hive-local`.
- Each worktree gets a port pair from 3100 up (app, hot reload). Caddy on port 80 routes
  `http://<name>.localhost` to it. Separate hostnames keep login cookies apart; ports alone
  would not.
- `<name>` is the branch name lowercased, with anything outside `a-z0-9-` turned into `-`.
  `hive` is the main checkout on port 3000 at `http://hive.localhost`.
- `.worktree.env` in each worktree holds its `DATABASE_URL`, port and origin. `.envrc`
  loads it, so any shell in the worktree talks to the right database.

## One-time setup

1. **Postgres role.** `hive-dev` must be allowed to create databases. The flake's `pg-start`
   does that. With any other Postgres, run once as a superuser:

   ```sh
   psql -h localhost -d postgres -c 'ALTER ROLE "hive-dev" CREATEDB'
   ```

2. **Template database.** Ask a maintainer for the scrubbed dump and load it:

   ```sh
   setup --template hive-template-YYYY-MM-DD.dump
   ```

   The dump holds real usernames and games. Keep it out of the repository (`*.dump` is
   ignored) and do not pass it on. Loading scrubs it again: every password becomes
   `hivegame`, emails become `<username>@hivegame.com`, chats, user blocks, chat mutes, push
   devices and email tokens are removed. Log in locally as any user with the password `hivegame`.

3. **Linux only: port 80.** Unprivileged processes may not bind port 80 on most Linux
   systems, so the proxy falls back to port 8080 there and URLs become
   `http://<name>.localhost:8080`. Set `HIVE_PROXY_PORT` to choose another port, or allow
   port 80 with `sysctl net.ipv4.ip_unprivileged_port_start=80`.

4. **Safari and curl only.** Chrome and Firefox resolve `*.localhost` by themselves. For
   Safari add a line per worktree to `/etc/hosts`; `setup` prints it.

## Commands

| Command | Does |
|---|---|
| `setup <branch> [--from <ref>]` | Checks out `<branch>` in a new worktree. Uses the local branch, else `origin/<branch>`, else creates it from `<ref>` (default `origin/main`). |
| `setup --ls` | Lists instances with branch, port, database size, running state, URL. |
| `setup --refresh <name>` | Replaces `hive-<name>` with a fresh copy of the template. |
| `setup --template <dump>` | Rebuilds `hive-template` from a dump and scrubs it. |
| `run <name> [serve\|watch]` | Serves the worktree behind its URL. Moves to a free port if its port is taken. |
| `run <name> shell` | A dev shell in the worktree with its environment. |
| `cleanup [<name>...]` | Lists worktrees, databases, routes and leftovers, lets you pick, asks, removes. |

`cleanup` keeps branches, asks before stopping a running server, and makes you type the
name before removing a worktree with uncommitted changes. `hive-local`, `hive-test` and
`hive-template` are never touched.

The commands are copied into the nix store when the dev shell builds, so after editing a
script run `direnv reload` (or re-enter `nix develop`) to use the new version.

Run the commands from the main checkout. `run` starts the build with the worktree's own
flake, so the branch you serve does not need these scripts. Your main checkout does.
