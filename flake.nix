{
  description = "Simple OAuth2 client for mail clients";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {self,  nixpkgs, flake-utils, rust-overlay, ... }:
  flake-utils.lib.eachDefaultSystem (system:
  let
    overlays = [ (import rust-overlay) ];
    pkgs = import nixpkgs {
      inherit system overlays;
    };
    mfauth = with pkgs; rustPlatform.buildRustPackage rec {
      nativeBuildInputs = [ pkg-config ];
      buildInputs = [ openssl ];
      pname = "mfauth";
      version = "0.1.0";
      src = ./.;
      cargoSha256 = "sha256-1Xrhadqx0BEGpNScEVd51rmh8LmKXn26ox5rcmY8tL4=";
    };
  in
  with pkgs;
  {
    defaultPackage = mfauth;
    devShell = mkShell {
      buildInputs = [
        rust-bin.stable.latest.default
        cargo-watch
        cargo-limit
        openssl.dev
        pkg-config
      ];
    };
  });
}
