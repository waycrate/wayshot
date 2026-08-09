{pkgs}: let
  deps = with pkgs; [
    pango
    libgbm
    libGL
    wayland
  ];
in
  pkgs.mkShell {
    name = "wayshot-dev-shell";
    strictDeps = true;
    nativeBuildInputs = with pkgs; [
      # Compilers
      cargo
      rustc

      # Tools
      pkg-config
      clippy
      rust-analyzer
      rustfmt
      scdoc
    ];
    buildInputs = deps;
    LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath deps;
  }
