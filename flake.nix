{
  description = "Development environment for wayshot";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Define the library path
        libPath = with pkgs; lib.makeLibraryPath [
          libgbm
          wayland
          # Add other libraries your project depends on here
        ];
      in
      {
        devShells.default = pkgs.mkShell rec {
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = with pkgs; [
            clang
            llvmPackages.bintools
            rustc
            cargo
            libGL.dev
            libgbm
            wayland.dev
          ];

          # Set up environment variables for Rust
          LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
          LD_LIBRARY_PATH = libPath;
          RUST_BACKTRACE = "1";

          shellHook = ''
            export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
          '';

          # Add precompiled libraries to rustc search path
          RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
            # Add libraries here (e.g., pkgs.libvmi)
          ]);

          # Add headers to bindgen search path
          BINDGEN_EXTRA_CLANG_ARGS =
            # Includes normal include path
            (builtins.map (a: ''-I"${a}/include"'') [
              pkgs.glibc.dev
              # Add dev libraries here (e.g., pkgs.libvmi.dev)
            ])
            # Includes with special directory paths
            ++ [
              ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
              ''-I"${pkgs.glib.dev}/include/glib-2.0"''
              ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
            ];
        };
      }
    );
}
