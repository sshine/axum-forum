{
  description = "axum-forum is a basic web forum using tokio and axum";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [ "x86_64-linux" ];

      # System-dependent parts go here [flake-parts]
      perSystem = { self', inputs', config, lib, pkgs, system, ... }: {
        packages.default = import ./default.nix { inherit pkgs; };
      };

      # System-independent parts go here [flake-parts]
      flake = {
        nixosModules.default = import ./service.nix;
      };
    };
}
