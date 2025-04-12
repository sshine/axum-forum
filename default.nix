{ pkgs ? import <nixpkgs> {} }:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
}
