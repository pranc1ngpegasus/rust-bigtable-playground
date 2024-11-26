{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix.url = "github:nix-community/fenix/monthly";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
        };
      in {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            libiconv
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          env = {};

          nativeBuildInputs = [toolchain];

          shellHook = ''
            export LIBRARY_PATH=${pkgs.libiconv}/lib:$LIBRARY_PATH
            export LIBRARY_PATH=$LIBRARY_PATH:/usr/lib:${pkgs.libiconv}/lib
            export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:/usr/lib:${pkgs.libiconv}/lib
          '';
        };
      }
    );
}
