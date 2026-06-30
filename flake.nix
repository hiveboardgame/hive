{
  description = "Hivegame.com's flake to setup everything";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        nokamute = pkgs.callPackage (pkgs.fetchFromGitHub {
          owner = "frisoft";
          repo = "nokamute";
          rev = "master";
          sha256 = "sha256-7Q2VuVexug0iqBXEzHfQ/c9q7TfjL56psGbq5sU2Nw4=";
        }) { };
        # Migration drift tool. Two entry points:
        #   on-checkout PREV NEW FLAG  - post-checkout hook; reverts migrations
        #                                the new branch lacks but the DB applied.
        #   fix [--dry-run]            - manual repair; reverts every migration
        #                                in the DB that the current branch lacks.
        # A migration's down.sql is not on disk once you've left its branch, so we
        # recover it from git (the prior commit, or any branch/tag for `fix`),
        # run it, then drop the row from diesel's ledger.
        migrations-tool = pkgs.writers.writePython3Bin "migrations-tool" { flakeIgnore = [ "E501" ]; } ''
          import argparse
          import os
          import shutil
          import subprocess
          import sys

          MIG_DIR = "db/migrations"
          PREFIX = "[migrations]"
          INITIAL = "00000000000000"  # diesel_initial_setup, always present


          def log(msg):
              print(f"{PREFIX} {msg}", file=sys.stderr)


          def git(*args):
              return subprocess.run(["git", *args], capture_output=True, text=True)


          def psql(db_url, sql, *extra):
              return subprocess.run(
                  ["psql", db_url, "-v", "ON_ERROR_STOP=1", *extra],
                  input=sql,
                  capture_output=True,
                  text=True,
              )


          def repo_root():
              res = git("rev-parse", "--show-toplevel")
              return res.stdout.strip() if res.returncode == 0 else None


          def load_database_url():
              url = os.environ.get("DATABASE_URL")
              if url:
                  return url
              if not os.path.exists(".env"):
                  return None
              with open(".env") as fh:
                  for line in fh:
                      line = line.strip()
                      if not line or line.startswith("#") or "=" not in line:
                          continue
                      key, _, value = line.partition("=")
                      if key.strip() == "DATABASE_URL":
                          return value.strip().strip('"').strip("'")
              return None


          def version_of(dir_name):
              # Diesel stores the version as digits only, e.g. dir
              # 2023-09-26-143912_users -> 20230926143912.
              prefix = dir_name.split("_", 1)[0]
              return "".join(c for c in prefix if c.isdigit())


          def migrations_in(ref):
              res = git("ls-tree", "-d", "--name-only", ref, f"{MIG_DIR}/")
              if res.returncode != 0:
                  return set()
              return {os.path.basename(p) for p in res.stdout.split() if p}


          def migrations_on_disk():
              if not os.path.isdir(MIG_DIR):
                  return set()
              return {e.name for e in os.scandir(MIG_DIR) if e.is_dir()}


          def applied_versions(db_url):
              res = psql(db_url, "select version from __diesel_schema_migrations;", "-tA")
              if res.returncode != 0:
                  return None
              return {ln.strip() for ln in res.stdout.splitlines() if ln.strip()}


          def setup():
              root = repo_root()
              if not root:
                  return None
              os.chdir(root)
              db_url = load_database_url()
              if not db_url:
                  log("DATABASE_URL unset, skipping")
                  return None
              if not shutil.which("psql"):
                  log("psql not found, skipping")
                  return None
              probe = psql(db_url, "select 1 from __diesel_schema_migrations limit 1;", "-tA")
              if probe.returncode != 0:
                  log("DB or migrations table unavailable, skipping")
                  return None
              return db_url


          def revert(db_url, version, dir_name, ref, dry_run):
              down = git("show", f"{ref}:{MIG_DIR}/{dir_name}/down.sql")
              if down.returncode != 0:
                  log(f"WARN no down.sql for {dir_name} in {ref[:12]}, skipping")
                  return
              if dry_run:
                  log(f"would revert {dir_name} (down.sql from {ref})")
                  return
              log(f"reverting {dir_name}")
              sql = (
                  "BEGIN;\n"
                  f"{down.stdout}\n"
                  "DELETE FROM __diesel_schema_migrations WHERE version = :'v';\n"
                  "COMMIT;\n"
              )
              res = psql(db_url, sql, "-v", f"v={version}")
              if res.returncode != 0:
                  log(f"WARN revert of {dir_name} failed: {res.stderr.strip()}")


          def locate(versions):
              # For each wanted version find a (dir, ref) whose tree holds its down.sql.
              found = {}
              refs = git(
                  "for-each-ref", "--format=%(refname)",
                  "refs/heads", "refs/remotes", "refs/tags",
              ).stdout.split()
              for ref in refs:
                  if not versions - set(found):
                      break
                  for dir_name in migrations_in(ref):
                      version = version_of(dir_name)
                      if version in versions and version not in found:
                          if git("cat-file", "-e", f"{ref}:{MIG_DIR}/{dir_name}/down.sql").returncode == 0:
                              found[version] = (dir_name, ref)
              return found


          def cmd_on_checkout(args):
              # flag == 1 is a branch checkout; 0 is a file checkout (ignore).
              if args.flag != "1" or args.prev == args.new:
                  return 0
              db_url = setup()
              if not db_url:
                  return 0
              only_old = migrations_in(args.prev) - migrations_in(args.new)
              if not only_old:
                  return 0
              applied = applied_versions(db_url) or set()
              # Newest first so later migrations come down before the ones they build on.
              for dir_name in sorted(only_old, reverse=True):
                  version = version_of(dir_name)
                  if version in applied:
                      revert(db_url, version, dir_name, args.prev, dry_run=False)
              log("done")
              return 0


          def cmd_fix(args):
              db_url = setup()
              if not db_url:
                  return 0
              current = {version_of(d) for d in migrations_on_disk()}
              applied = applied_versions(db_url)
              if applied is None:
                  return 0
              orphans = applied - current - {INITIAL}
              if not orphans:
                  log("DB matches current branch, nothing to revert")
                  return 0
              located = locate(orphans)
              # Newest first so later migrations come down before the ones they build on.
              for version in sorted(orphans, reverse=True):
                  match = located.get(version)
                  if not match:
                      log(f"WARN no branch/tag has down.sql for {version}, leaving as-is")
                      continue
                  dir_name, ref = match
                  revert(db_url, version, dir_name, ref, dry_run=args.dry_run)
              log("dry-run complete" if args.dry_run else "done")
              return 0


          def main(argv):
              parser = argparse.ArgumentParser(prog="migrations-tool")
              sub = parser.add_subparsers(dest="cmd", required=True)

              p_co = sub.add_parser("on-checkout", help="post-checkout hook entry point")
              p_co.add_argument("prev")
              p_co.add_argument("new")
              p_co.add_argument("flag")
              p_co.set_defaults(func=cmd_on_checkout)

              p_fix = sub.add_parser("fix", help="revert migrations not on the current branch")
              p_fix.add_argument("--dry-run", action="store_true", help="show what would be reverted")
              p_fix.set_defaults(func=cmd_fix)

              args = parser.parse_args(argv)
              return args.func(args)


          if __name__ == "__main__":
              sys.exit(main(sys.argv[1:]))
        '';
        # Thin git-hook shim that drives the tool with the args git passes.
        post-checkout-hook = pkgs.writeShellScript "post-checkout" ''
          exec ${migrations-tool}/bin/migrations-tool on-checkout "$@"
        '';
        aliases = [
          (pkgs.writeShellScriptBin "server" ''
            #!/usr/bin/env bash
            cargo leptos watch
          '')
          (pkgs.writeShellScriptBin "migration" ''
            #!/usr/bin/env bash
            cd db;
            diesel migration "$@"
            cd ..
            echo "You are welcome! ┬─┬﻿ ノ( ゜-゜ノ) "
          '')
          (pkgs.writeShellScriptBin "fix" ''
            #!/usr/bin/env bash
            LEPTOS_OUTPUT_NAME="dev" cargo clippy --fix --all-features
            echo "You are welcome! ⊂(◉‿◉)つ"
          '')
          (pkgs.writeShellScriptBin "format" ''
            #!/usr/bin/env bash
            leptosfmt apis -q
            cargo fmt
            echo $(git diff --shortstat)
            echo "You are welcome! ٩( ๑╹ ꇴ╹)۶"
          '')
          (pkgs.writeShellScriptBin "pg-start" ''
            #!/usr/bin/env bash
            if [ -d ".pg" ]; then
              pg_ctl -D ".pg/data" -l ".pg/postgresql.log" -o "-k $PWD/.pg/run" start
            else
              initdb -D ".pg/data"
              mkdir ".pg/run"
              pg_ctl -D ".pg/data" -l ".pg/postgresql.log" -o "-k $PWD/.pg/run" start
            fi

            if ! psql -h localhost -d postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='hive-dev'" | grep -q 1; then
              createuser -h localhost hive-dev
            fi

            for db in hive-local hive-test; do
              if ! psql -h localhost -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname='$db'" | grep -q 1; then
                createdb -h localhost -O hive-dev "$db"
              fi
            done

            echo "PotgreSQL started (log: .pg/postgresql.log)"
          '')
          (pkgs.writeShellScriptBin "pg-stop" ''
            #!/usr/bin/env bash
            pg_ctl -D "$PWD/.pg/data" -l "$PWD/.pg/postgresql.log" -o "-k $PWD/.pg/run" stop
            echo "PotgreSQL stopped"
          '')
          (pkgs.writeShellScriptBin "database" ''
            #!/usr/bin/env bash
            psql postgres://hive-dev@localhost:/hive-local
          '')
          (pkgs.writeShellScriptBin "hive-hydra" ''
            #!/usr/bin/env bash
            cargo run --package hive-hydra -- --config hive-hydra/hive-hydra.yaml
          '')
          (pkgs.writeShellScriptBin "hydra" ''
            #!/usr/bin/env bash
            cargo run --package hive-hydra -- --config hive-hydra/hive-hydra.prod.yaml
          '')
          (pkgs.writeShellScriptBin "migration-hook" ''
            #!/usr/bin/env bash
            set -euo pipefail
            hooks="$(git rev-parse --git-path hooks)"
            ln -sf ${post-checkout-hook} "$hooks/post-checkout"
            echo "Installed: branch-migration auto-revert runs on git checkout."
            echo "Disable with: rm '$hooks/post-checkout'"
          '')
          (pkgs.writeShellScriptBin "migration-fix" ''
            #!/usr/bin/env bash
            exec ${migrations-tool}/bin/migrations-tool fix "$@"
          '')
        ];
      in
      with pkgs;
      {
        devShells.default = mkShell rec {
          buildInputs =
            [
              diesel-cli
              (diesel-cli.override {
                sqliteSupport = false;
                mysqlSupport = false;
                postgresqlSupport = true;
              })
              pkg-config
              cacert
              cargo-make
              cargo
              trunk
              dart-sass
              leptosfmt
              postgresql
              flyctl
              cargo-leptos
              tailwindcss_4
              openssl
              # nokamute # The AI used by hive-hydra - temporarily disabled due to darwin SDK issue
              (rust-bin.selectLatestNightlyWith (
                toolchain:
                toolchain.default.override {
                  extensions = [
                    "rust-src"
                    "rust-analyzer"
                  ];
                  targets = [ "wasm32-unknown-unknown" ];
                }
              ))
            ]
            ++ aliases;
          shellHook = ''
            export CARGO_TARGET_DIR="$PWD/.cargo/target"
            # Install wasm-bindgen-cli at the version matching our Cargo.lock
            WASM_BINDGEN_VERSION="0.2.108"
            if ! command -v wasm-bindgen &> /dev/null || [[ "$(wasm-bindgen --version 2>/dev/null | cut -d' ' -f2)" != "$WASM_BINDGEN_VERSION" ]]; then
              echo "Installing wasm-bindgen-cli $WASM_BINDGEN_VERSION..."
              cargo install wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION" --quiet
            fi
            echo "Welcome to hivegame.com"
            echo "'server' to start everything"
            echo "'hive-hydra' to start hive-hydra for playing with bots"
            echo "'hydra' to start hydra productively"
            echo "'format' to make the code look nice"
            echo "'pg-start' to start PosgreSQL"
            echo "'migration' to 'run', 'revert', ... DB changes"
            echo "'pg-stop' to stop PosgreSQL"
            echo "'database' to connect to the PosgreSQL database"
            echo "'migration-hook' to install the branch-migration auto-revert git hook"
            echo "'migration-fix [--dry-run]' to revert migrations not on the current branch"
          '';
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };
      }
    );
}
