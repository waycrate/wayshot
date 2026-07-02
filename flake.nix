{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      forAllSystems =
        callback:
        nixpkgs.lib.genAttrs [
          "x86_64-linux"
          "aarch64-linux"
        ] (system: callback nixpkgs.legacyPackages.${system});

      mkDeps =
        pkgs: with pkgs; [
          pango
          libgbm
          libGL
          wayland
        ];
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          inherit (pkgs) lib;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
            pname = "wayshot";
            version = "${(builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version}-git";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = mkDeps pkgs;

            meta = {
              description = "Screenshot crate for wlroots based compositors implementing the zwlr_screencopy_v1 protocol.";
              homepage = "https://crates.io/crates/wayshot";
              changelog = "https://github.com/waycrate/wayshot/releases/tag/v${finalAttrs.version}";
              license = with lib.licenses; [
                bsd2
                gpl3Only
              ];
              mainProgram = "wayshot";
              platforms = lib.platforms.linux;
            };
          });
        }
      );

      devShells.default = forAllSystems (
        pkgs:
        let
          inherit (pkgs) lib;
        in
        pkgs.mkShell {
          strictDeps = true;
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
            pkg-config
          ];
          buildInputs = mkDeps pkgs;
          LD_LIBRARY_PATH = lib.makeLibraryPath (mkDeps pkgs);
        }
      );
    };
}
