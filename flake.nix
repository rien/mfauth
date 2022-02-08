{
  description = "Simple CLI client to request and manage OAuth2 tokens";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        cargoTOML = pkgs.lib.importTOML ./Cargo.toml;
      in
      rec {
        packages = {
          mfauth = with pkgs; rustPlatform.buildRustPackage {
            nativeBuildInputs = [ pkg-config ];
            buildInputs = [ openssl ];
            pname = cargoTOML.package.name;
            version = cargoTOML.package.version;
            src = ./.;
            cargoSha256 = "sha256-6Jf7KlPRTrllXqz+XoUqNXp36UkNXheaB0ENF+fXIpg=";
          };
        };
        defaultPackage = packages.mfauth;
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
            cargo-watch
            cargo-limit
            openssl.dev
            pkg-config
          ];
        };
      });
}
