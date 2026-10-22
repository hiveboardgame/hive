# Hive worktrees

Run branches side by side with separate checkouts, databases, build outputs, and URLs.
For first use, follow [Setup](#setup).

## Everyday commands

Run these from your main checkout:

| Command | Use |
|---|---|
| `hive-setup <branch>` | Create a worktree for an existing or new branch. |
| `hive-run <name> watch` | Build and serve with hot reload. Omit `watch` to serve without watching. |
| `hive-run <name> shell` | Open a shell with that instance's environment. |
| `hive-setup --ls` | List instances and their URLs. |
| `hive-setup --refresh <name>` | Replace the development database with a fresh copy of its saved template. |
| `hive-cleanup` | Select environments to remove. |

Use the instance name printed by setup. `hive` means the main checkout on its current
branch. Worktrees live in `../hive-worktrees/<name>` by default. Inside a worktree or
the main checkout, `hive-run` can leave out the name, as in `hive-run watch`. Each
command has `--help` for additional options.

Before running Cargo, Diesel, or SQL, enter `hive-run <name> shell`. Changing directory
alone does not load the environment. Exit that shell before selecting another instance.

Ctrl-C stops the app or watcher. The shared Caddy proxy stays running; stop it with
`caddy stop --address localhost:2019` when no instance needs it.

## Setup

### Native

Install the project's Rust toolchain and build dependencies, including cargo-leptos,
the `wasm32-unknown-unknown` target, wasm-bindgen-cli, Sass, Tailwind CSS, pkg-config,
OpenSSL, and libpq development libraries. The scripts also need Bash 4+, Git,
PostgreSQL clients, Caddy, curl, lsof, and standard Unix utilities. `hive-cleanup` uses
the GitHub CLI (`gh`, logged in) when available to spot merged pull requests.

```sh
./scripts/worktree/install.sh
```

Add `~/.local/bin` to `PATH`. For native use, leave the repository's Nix `.envrc` disabled.

### Nix

Enter `nix develop` or use direnv. The dev shell provides the commands and dependencies.
After editing scripts, run `direnv reload` or re-enter `nix develop`.

### Database

Start with the [project's PostgreSQL setup](../../README.md#development). Grant
`hive-dev` permission to create databases, once, as a PostgreSQL superuser
(the Nix `pg-start` command already does this):

```sh
psql -h localhost -d postgres -c 'ALTER ROLE "hive-dev" CREATEDB'
```

Ask a maintainer for a template dump and load it with a compatible `pg_restore` version:

```sh
hive-setup --template hive-template-YYYY-MM-DD.dump
```

Loading [scrubs private data](scrub.sql) and sets local passwords to `hivegame`.
Dumps still contain real usernames and games: keep them private and out of Git.
Use `--scrubbed` only for dumps already scrubbed by a trusted process.

Each worktree gets a copy of the template and a separate test database. App startup
applies pending migrations; replacing a template leaves existing worktree data alone.

### Local settings

Keep private app settings in the main checkout's ignored `.worktree.env`, with file
permissions set to `600`. Worktrees inherit these settings and select their own
databases, ports, and output paths.

To override defaults, export these before running the commands:

- `HIVE_WORKTREES_DIR`: parent directory for worktrees.
- `HIVE_PROXY_PORT`: Caddy's port (default: 80 or 8080).

## Other devices

```sh
hive-run <name> watch --share
```

Open the printed LAN or Tailnet URL on a device that can reach your machine.
Sharing needs DNS that allows private IP addresses and access to the proxy port.
Additional development origins are ignored in release builds, which accept only
the configured `APP_ORIGIN`; use the printed proxy URL for those builds.
To stop sharing, stop the watcher, then run it again without `--share`.

## Cleanup

Cleanup removes selected worktrees, their databases, and proxy routes. It shows
estimated sizes and asks before deleting, stopping a running app, or discarding
uncommitted changes. Branches, main databases, and seed templates are kept.

Databases from older setups carry no owner, and once their worktree is gone nothing ties
them to it. Cleanup lists any such `hive-*` database as `db:<name>` with no owner. It
never preselects them, since another clone of the repository may use them.

Without names, cleanup fetches `main` and preselects clean worktrees whose work is
merged. A worktree counts as merged when:

- it has commits of its own and merging them into `origin/main` would change no files
  (needs Git 2.38 or later),
- its `HEAD` is already in `origin/main` and either arrived through a merge commit or
  belongs to a branch with a merged pull request, or
- GitHub has a pull request merged into `main` whose head contains its `HEAD`.

A `HEAD` on main's own history without a merged pull request looks like a fresh
worktree, so branches pushed straight onto main are never picked. Squash merges whose
lines main has changed since are only found through GitHub, which uses `gh`; without
it, or when offline, the other checks still run. Naming instances on the command line
skips these checks.
