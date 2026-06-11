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
          (pkgs.writeShellScriptBin "server-mobile" ''
            #!/usr/bin/env bash
            # cargo-leptos watch, but bound to all interfaces so emulators /
            # simulators / physical devices can reach the backend over the
            # host's LAN IP. The regular `server` alias stays on 127.0.0.1
            # so default dev isn't exposed to the LAN.
            export LEPTOS_SITE_ADDR="0.0.0.0:3000"
            exec cargo leptos watch
          '')
          (pkgs.writeShellScriptBin "trunk-mobile" ''
            #!/usr/bin/env bash
            set -euo pipefail
            # Single trunk dev server for both iOS sim and Android emulator
            # (culex). Bakes the host's LAN-IP into the WASM bundle so the
            # same URL works on every mobile target — no more split between
            # localhost (iOS) and 10.0.2.2 (Android).
            #
            # LEPTOS_SERVER_URL / LEPTOS_WS_URL are option_env! reads in
            # apis/src/lib.rs and apis/src/app.rs, so they have to be set
            # at the trunk-build step (not at runtime).
            iface=$(route get default 2>/dev/null | awk '/interface:/{print $2; exit}')
            if [ -z "''${iface:-}" ]; then
              echo "trunk-mobile: no default route — connect Wi-Fi/Ethernet first." >&2
              exit 1
            fi
            host_ip=$(ipconfig getifaddr "$iface" 2>/dev/null || true)
            if [ -z "$host_ip" ]; then
              echo "trunk-mobile: $iface has no IPv4 — try toggling Wi-Fi." >&2
              exit 1
            fi
            echo "trunk-mobile: backend at http://$host_ip:3000 (interface $iface)"
            echo "             ensure 'server-mobile' is running to accept LAN traffic"
            export LEPTOS_SERVER_URL="http://$host_ip:3000"
            export LEPTOS_WS_URL="ws://$host_ip:3000/ws/"
            cd "$(git rev-parse --show-toplevel)/apis"
            exec trunk serve "$@"
          '')
          (pkgs.writeShellScriptBin "tauri-android" ''
            #!/usr/bin/env bash
            # Wraps `cargo tauri android <args>` from the culex/ directory.
            # Auto-sets TAURI_DEV_HOST to the host's LAN IP so the emulator's
            # webview points at the same trunk-mobile server iOS uses — no
            # more 10.0.2.2-specific tauri.android.conf.json.
            # ANDROID_HOME / NDK_HOME / JAVA_HOME come from the shellHook.
            if [ -z "$ANDROID_HOME" ]; then
              echo "tauri-android: ANDROID_HOME is unset — install Android Studio + SDK first." >&2
              exit 1
            fi
            if [ -z "''${TAURI_DEV_HOST:-}" ]; then
              iface=$(route get default 2>/dev/null | awk '/interface:/{print $2; exit}')
              if [ -n "''${iface:-}" ]; then
                export TAURI_DEV_HOST=$(ipconfig getifaddr "$iface" 2>/dev/null || true)
              fi
            fi
            if [ -n "''${TAURI_DEV_HOST:-}" ]; then
              echo "tauri-android: TAURI_DEV_HOST=$TAURI_DEV_HOST"
            fi
            cd "$(git rev-parse --show-toplevel)/culex"
            exec cargo tauri android "$@"
          '')
          (pkgs.writeShellScriptBin "tauri-ios" ''
            #!/usr/bin/env bash
            # Wraps `cargo tauri ios <args>` with:
            #  1. DEVELOPER_DIR pointing at real Xcode (so xcrun/simctl work).
            #     Can't be set globally — breaks nix's clang wrapper for macOS
            #     host builds (Android cross-compile, build scripts, etc.).
            #  2. CC/CXX/AR/linker pointing at Xcode's UNWRAPPED clang for
            #     iOS targets. Nix's cc-wrapper auto-injects -mmacos-version-min
            #     which conflicts with -mios-simulator-version-min and fails
            #     iOS cross-compile (e.g. objc2-exception-helper's try_catch.m).
            if [ ! -d /Applications/Xcode.app/Contents/Developer ]; then
              echo "tauri-ios: Xcode not found at /Applications/Xcode.app — install from the App Store first." >&2
              exit 1
            fi
            export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer

            # Nix's devShell puts xcbuild's stub `xcrun` (in
            # /nix/store/...-xcbuild-.../bin) on PATH ahead of Apple's
            # /usr/bin/xcrun. The stub answers `xcrun -f <tool>` lookups but
            # cannot talk to CoreSimulator — so `xcrun simctl list` fails with
            # code 1 and tauri reports "Failed to detect connected iOS
            # Simulator devices". Prepend /usr/bin so Apple's xcrun wins
            # (which itself routes through DEVELOPER_DIR set above).
            export PATH="/usr/bin:$PATH"

            sim_clang=$(xcrun --sdk iphonesimulator -f clang)
            sim_clangpp=$(xcrun --sdk iphonesimulator -f clang++)
            sim_ar=$(xcrun --sdk iphonesimulator -f ar)
            dev_clang=$(xcrun --sdk iphoneos -f clang)
            dev_clangpp=$(xcrun --sdk iphoneos -f clang++)
            dev_ar=$(xcrun --sdk iphoneos -f ar)

            export CC_aarch64_apple_ios_sim="$sim_clang"
            export CXX_aarch64_apple_ios_sim="$sim_clangpp"
            export AR_aarch64_apple_ios_sim="$sim_ar"
            export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER="$sim_clang"

            export CC_aarch64_apple_ios="$dev_clang"
            export CXX_aarch64_apple_ios="$dev_clangpp"
            export AR_aarch64_apple_ios="$dev_ar"
            export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$dev_clang"

            # Auto-set TAURI_DEV_HOST to the host's LAN IP so the iOS sim's
            # webview points at the same URL Android uses (trunk-mobile). The
            # iOS sim could reach localhost too, but using LAN IP everywhere
            # makes the dev flow uniform and lets a physical iPhone work via
            # the same command.
            if [ -z "''${TAURI_DEV_HOST:-}" ]; then
              iface=$(route get default 2>/dev/null | awk '/interface:/{print $2; exit}')
              if [ -n "''${iface:-}" ]; then
                export TAURI_DEV_HOST=$(ipconfig getifaddr "$iface" 2>/dev/null || true)
              fi
            fi
            if [ -n "''${TAURI_DEV_HOST:-}" ]; then
              echo "tauri-ios: TAURI_DEV_HOST=$TAURI_DEV_HOST"
            fi

            cd "$(git rev-parse --show-toplevel)/culex"
            exec cargo tauri ios "$@"
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
                  targets = [
                    "wasm32-unknown-unknown"
                  ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                    # HiveGame mobile app (culex / Tauri) — only meaningful on
                    # macOS (iOS requires Xcode; Android is doable on Linux but
                    # the primary dev path is Mac).
                    "aarch64-linux-android"
                    "armv7-linux-androideabi"
                    "i686-linux-android"
                    "x86_64-linux-android"
                    "aarch64-apple-ios"
                    "aarch64-apple-ios-sim"
                  ];
                }
              ))
            ]
            ++ aliases;
          shellHook = ''
            export CARGO_TARGET_DIR="$PWD/.cargo/target"
            # NOTE on iOS Tauri (culex): we do NOT export DEVELOPER_DIR
            # globally. Setting it to Xcode breaks nix's clang-wrapper for
            # host build-script compilation (libSystem unresolved). For iOS
            # builds, prefix the command instead:
            #   DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer cargo tauri ios dev
            # The `tauri-ios` alias below wraps this.
            # For culex (Android Tauri): pick up Android Studio's SDK/NDK/JBR
            # when present. Tauri's android init/dev needs ANDROID_HOME and
            # NDK_HOME; gradle wants JAVA_HOME; adb/emulator on PATH is handy.
            if [ -d "$HOME/Library/Android/sdk" ]; then
              export ANDROID_HOME="$HOME/Library/Android/sdk"
              export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
              if [ -d "$ANDROID_HOME/ndk" ] && [ -n "$(ls "$ANDROID_HOME/ndk" 2>/dev/null)" ]; then
                export NDK_HOME="$ANDROID_HOME/ndk/$(ls "$ANDROID_HOME/ndk" | sort -V | tail -1)"
              fi
            fi
            if [ -d "/Applications/Android Studio.app/Contents/jbr/Contents/Home" ]; then
              export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
            fi
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
            echo "HiveGame mobile (culex):"
            echo "  'server-mobile'  — cargo leptos watch on 0.0.0.0:3000 (LAN-reachable backend)"
            echo "  'trunk-mobile'   — trunk serve, bakes host's LAN IP into the WASM bundle"
            echo "  'tauri-ios dev' / 'tauri-android dev' — launch the mobile app"
          '';
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };
      }
    );
}
