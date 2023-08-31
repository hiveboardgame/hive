{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell rec {
  nativeBuildInputs = [
    pkgconfig
    llvmPackages.bintools # To use lld linker
  ];
  buildInputs = [
    dart-sass
    openssl
    postgresql
  ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
}
