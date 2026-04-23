{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { pkgs, system, ... }:
        let
          deps = with pkgs; [ pango libgbm libGL wayland ];
        in
        {
          packages.default = pkgs.rustPlatform.buildRustPackage {
            pname = "wayshot";
            version = "1.4.6";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = deps;
          };

          devShells.default = pkgs.mkShell {
            strictDeps = true;
            nativeBuildInputs = with pkgs; [
              cargo
              rustc
              rust-analyzer
              clippy
              rustfmt
              pkg-config
            ];
            buildInputs = deps;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath deps;
          };
        };
    };
}
