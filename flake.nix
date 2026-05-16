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
          (pkgs.writeShellScriptBin "trunk-ios" ''
            #!/usr/bin/env bash
            # Trunk dev server for the Apiary iOS simulator. Defaults are
            # fine: WASM hits the backend at localhost:3000, which the iOS
            # simulator shares with the host.
            cd "$(git rev-parse --show-toplevel)/apis"
            exec trunk serve "$@"
          '')
          (pkgs.writeShellScriptBin "trunk-android" ''
            #!/usr/bin/env bash
            # Trunk dev server for the Apiary Android emulator. The emulator
            # reaches the host machine via 10.0.2.2, so bake that into the
            # WASM bundle (LEPTOS_SERVER_URL / LEPTOS_WS_URL are read at
            # compile time in apis/src/lib.rs and apis/src/app.rs).
            export LEPTOS_SERVER_URL="http://10.0.2.2:3000"
            export LEPTOS_WS_URL="ws://10.0.2.2:3000/ws/"
            cd "$(git rev-parse --show-toplevel)/apis"
            exec trunk serve "$@"
          '')
          (pkgs.writeShellScriptBin "tauri-android" ''
            #!/usr/bin/env bash
            # Wraps `cargo tauri android <args>` from the apiary/ directory.
            # tauri.android.conf.json overrides devUrl to http://10.0.2.2:8080
            # so the emulator's webview can reach the host's trunk server.
            # ANDROID_HOME / NDK_HOME / JAVA_HOME come from the shellHook.
            if [ -z "$ANDROID_HOME" ]; then
              echo "tauri-android: ANDROID_HOME is unset — install Android Studio + SDK first." >&2
              exit 1
            fi
            cd "$(git rev-parse --show-toplevel)/apiary"
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

            cd "$(git rev-parse --show-toplevel)/apiary"
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
                    # Apiary mobile app (Tauri) — only meaningful on macOS
                    # (iOS requires Xcode; Android is doable on Linux but
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
            # NOTE on iOS Tauri (Apiary): we do NOT export DEVELOPER_DIR
            # globally. Setting it to Xcode breaks nix's clang-wrapper for
            # host build-script compilation (libSystem unresolved). For iOS
            # builds, prefix the command instead:
            #   DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer cargo tauri ios dev
            # The `tauri-ios` alias below wraps this.
            # For Apiary (Android Tauri): pick up Android Studio's SDK/NDK/JBR
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
            echo "Apiary mobile:"
            echo "  'trunk-ios' / 'trunk-android' — trunk serve with the right backend URL baked in"
            echo "  'tauri-ios dev' / 'tauri-android dev' — launch the mobile app"
          '';
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };
      }
    );
}
