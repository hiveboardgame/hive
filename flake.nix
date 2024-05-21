{
  description = "Hivegame.com's flake to setup everything";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        aliases = [(pkgs.writeShellScriptBin "server" ''
          #!/usr/bin/env bash
          mold -run cargo leptos watch
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
        (pkgs.writeShellScriptBin "testware" ''
          #!/usr/bin/env bash
          echo "Running tests"
          pushd apis/end2end
          playwright test "$@"
          echo "Tests finished. run show-report to see the results"
          popd
        '')
          (pkgs.writeShellScriptBin "show-report" ''
          #!/usr/bin/env bash
          pushd apis/end2end
          playwright show-report
          popd
        '')
        ];
      in
      with pkgs;
      {
        devShells.default = mkShell rec {
          buildInputs = [
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
            mold
            dart-sass
            rustfmt
            leptosfmt
            postgresql
            flyctl
            playwright-test
            playwright-driver
            (rust-bin.selectLatestNightlyWith( toolchain: toolchain.default.override {
              extensions= [ "rust-src" "rust-analyzer" ];
              targets = [ "wasm32-unknown-unknown" ];
            }))
          ] ++ pkgs.lib.optionals pkg.stdenv.isDarwin [
            darwin.apple_sdk.frameworks.SystemConfiguration
          ] ++ aliases;
          shellHook = ''
            echo "Welcome to hivegame.com"
            echo "'server' to start everything"
            echo "'format' to make the code look nice"
            echo "'migration' to 'run', 'revert', ... DB changes"
            echo "'testware' to run end2end tests (start the server in a separate terminal)"
          '';
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          PLAYWRIGHT_BROWSERS_PATH=pkgs.playwright-driver.browsers;
        };
      }
    );
}
