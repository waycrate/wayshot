{
  description = "Wayshot devel";

  inputs = { nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable"; };

  outputs = { self, nixpkgs, ... }:
    let
      pkgsFor = system:
        import nixpkgs {
          inherit system;
          overlays = [ ];
        };

      targetSystems = [ "aarch64-linux" "x86_64-linux" ];
    in {
      devShells = nixpkgs.lib.genAttrs targetSystems (system:
        let pkgs = pkgsFor system;
        in {
          default = pkgs.mkShell {
            name = "Wayshot-devel";
            nativeBuildInputs = with pkgs; [
              # Compilers
              clang
              cmake
              meson
              ninja
              cargo
              rustc
              scdoc

              # Libs
              inih
              pipewire
              wayland
              systemd
              mesa
              wayland-protocols

              # Tools
              rustfmt
              clippy
              pkg-config
              gdb
              gnumake
              rust-analyzer
              strace
              valgrind
              wayland-scanner
            ];
          };
        });
    };
}
