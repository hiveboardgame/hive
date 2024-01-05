{
  description = "Flake with Rust Env";

  inputs = {
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = inputs@{ self, nixpkgs-unstable, treefmt-nix, ... }:
    let
      forAllSystems = nixpkgs-unstable.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs-unstable.legacyPackages."${system}";
        in
        {
          default = pkgs.mkShell {
            CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";

            buildInputs = with pkgs; [
              cargo
              rustc
              rustfmt

              mold
              docker

              dart-sass
              openssl
              postgresql
              pkg-config

              flyctl

              git
            ];
          };
        }
      );

      formatter = forAllSystems (system:
        let
          pkgs = nixpkgs-unstable.legacyPackages."${system}";
        in
        treefmt-nix.lib.mkWrapper
          pkgs
          {
            projectRootFile = "flake.nix";
            programs.nixpkgs-fmt.enable = true;
            programs.rustfmt.enable = true;
          }
      );
    };
}
