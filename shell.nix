# shell.nix

with (import <nixpkgs> {});

let
    libPath = with pkgs; lib.makeLibraryPath [
    #libGL.dev
    libgbm
    #mesa.dev
    #egl-wayland.dev
    wayland 
      # load external libraries that you need in your rust project here
    ];
  moz_overlay = import /home/Kihsir/Git_Clone/nixpkgs-mozilla/rust-overlay.nix;
  rust_src_overlay = import /home/Kihsir/Git_Clone/nixpkgs-mozilla/rust-src-overlay.nix;
  # Import nixpkgs with both overlays included
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay rust_src_overlay ]; };

in 
  mkShell {
    name = "moz_overlay_shell";
    buildInputs = [ 
      stdenv.cc
      libGL.dev
      pkg-config
      wlr-protocols
      #libegl1-mesa-dev
      libGLU.dev
      mesa.dev
      libgbm
      #zstd
      #libxkbcommon
      #vulkan-loader
      #xorg.libX11
      #xorg.libXcursor
      #xorg.libXi
      #xorg.libXrandr
      #alsa-lib.dev
      wayland.dev
      #egl-wayland.dev
      udev.dev
      # to use the latest nightly:
      nixpkgs.latest.rustChannels.nightly.rust
    ];
  LD_LIBRARY_PATH = libPath ;
  RUST_BACKTRACE = 1;
  shellHook = ''
    # Set the RUST_SRC_PATH environment variable to the rust-src location
    export RUST_SRC_PATH="${nixpkgs.latest.rustChannels.nightly.rust-src}/lib/rustlib/src/rust/library"
    export LD_LIBRARY_PATH=${pkgs.wayland.dev}/lib:$LD_LIBRARY_PATH
  '';
  BINDGEN_EXTRA_CLANG_ARGS = (builtins.map (a: ''-I"${a}/include"'') [
    wayland.dev
    # Add include paths for other libraries here
  ])
  ++ [
    # Special directories
  ];
  }


