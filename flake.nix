{
  description = "Development environment for wayshot";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    moz-overlay.url = "github:mozilla/nixpkgs-mozilla";
  };

  outputs = { self, nixpkgs, moz-overlay }:
    let
      # Define the system architecture (e.g., "x86_64-linux")
      system = "x86_64-linux";

      # Import nixpkgs with the Mozilla overlay
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ moz-overlay.overlay ];
      };

      # Define the library path
      libPath = with pkgs; lib.makeLibraryPath [
        libgbm
        wayland
      ];

    in {
      devShells.${system}.default = pkgs.mkShell {
        name = "wayshot-dev-shell";

        buildInputs = [
          pkgs.libGL.dev
          pkgs.libgbm
          pkgs.pkg-config
          pkgs.wayland.dev
          pkgs.latest.rustChannels.nightly.rust
        ];

        LD_LIBRARY_PATH = libPath;
        RUST_BACKTRACE = "1";

        shellHook = ''
          export RUST_SRC_PATH="${pkgs.latest.rustChannels.nightly.rust-src}/lib/rustlib/src/rust/library"
        '';

        BINDGEN_EXTRA_CLANG_ARGS = (builtins.map (a: ''-I"${a}/include"'') [
          # Add include paths for other libraries here
        ]) ++ [
          # Special directories
        ];
      };
    };
}
