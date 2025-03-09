# shell.nix

with (import <nixpkgs> { });

let
  libPath =
    with pkgs;
    lib.makeLibraryPath [
      libgbm
      wayland
      # You can load external libraries that you need in your rust project here
    ];
  moz_overlay = import (builtins.fetchTarball "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz");
  # Here I have used pkgs-mozilla overlays for my use case however you can use any other method you prefer!!

  nixpkgs = import <nixpkgs> {
    overlays = [
      moz_overlay
    ];
  };

in
mkShell {
  name = "moz_overlay_shell";
  buildInputs = [
    libGL.dev
    libgbm
	pkg-config
    wayland.dev
    nixpkgs.latest.rustChannels.nightly.rust
  ];
  LD_LIBRARY_PATH = libPath;
  RUST_BACKTRACE = 1;
  shellHook = ''
    # Set the RUST_SRC_PATH environment variable to the rust-src location if required
    export RUST_SRC_PATH="${nixpkgs.latest.rustChannels.nightly.rust-src}/lib/rustlib/src/rust/library"
  '';
  BINDGEN_EXTRA_CLANG_ARGS =
    (builtins.map (a: ''-I"${a}/include"'') [
      # Add include paths for other libraries here
    ])
    ++ [
      # Special directories
    ];
}
