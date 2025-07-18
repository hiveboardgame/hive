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
              createuser -h localhost hive-dev && createdb -h localhost -O hive-dev hive-local
            fi
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
              rustfmt
              leptosfmt
              postgresql
              flyctl
              cargo-leptos
              tailwindcss
              openssl
              nokamute # The AI used by hive-hydra
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
            ++ pkgs.lib.optionals pkg.stdenv.isDarwin [
              darwin.apple_sdk.frameworks.SystemConfiguration
            ]
            ++ aliases;
          shellHook = ''
            export CARGO_TARGET_DIR="$PWD/.cargo/target"
            echo "Welcome to hivegame.com"
            echo "'server' to start everything"
            echo "'hive-hydra' to start hive-hydra for playing with bots"
            echo "'hydra' to start hydra productively"
            echo "'format' to make the code look nice"
            echo "'pg-start' to start PosgreSQL"
            echo "'migration' to 'run', 'revert', ... DB changes"
            echo "'pg-stop' to stop PosgreSQL"
            echo "'database' to connect to the PosgreSQL database"
          '';
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };
      }
    );
}
