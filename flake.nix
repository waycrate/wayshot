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
              cargo
              rustc
              scdoc

              # Libs
              wayland-protocols
              wayland

              # Tools
              wayland-scanner
              clippy
              gdb
              gnumake
              rust-analyzer
              rustfmt
              strace
              valgrind
            ];
          };
        });
    };
}
