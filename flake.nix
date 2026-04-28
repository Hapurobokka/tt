{
  description = "tt-map — terminal TTRPG map tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            clippy
            rustfmt
            rustPlatform.rustLibSrc
            evcxr
            irust
            bacon
          ];

          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          RUST_BACKTRACE = "full";
        };
      }
    );
}
