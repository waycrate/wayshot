{
  description = "wayshot devel and build";

  # Unstable required until Rust 1.85 (2024 edition) is on stable
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  # shell.nix compatibility
  inputs.flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";

  outputs = { self, nixpkgs, ... }:
    let
      # System types to support.
      targetSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      # Helper function to generate an attrset '{ x86_64-linux = f "x86_64-linux"; ... }'.
      forAllSystems = nixpkgs.lib.genAttrs targetSystems;
    in {
      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            strictDeps = true;
            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
            nativeBuildInputs = with pkgs; [
              cargo
              rustc
              pkg-config

              rustfmt
              clippy
              rust-analyzer

              scdoc
            ];

            buildInputs = with pkgs; [
              wayland
              libGL # libegl
              glib
              pango
              libgbm
            ];
          };
        }
      );
    };
}
