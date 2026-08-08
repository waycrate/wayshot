{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  installShellFiles,
  scdoc,
  pango,
  wayland,
  jpegSupport ? true,
  pnmSupport ? true,
  qoiSupport ? true,
  webpSupport ? true,
  avifSupport ? true,
  jxlSupport ? true,
  clipboardSupport ? true,
  colorPickerSupport ? true,
  completionsSupport ? true,
  loggerSupport ? true,
  notificationsSupport ? true,
  selectorSupport ? true,
}:
rustPlatform.buildRustPackage rec {
  pname = "wayshot";
  version = "${
    (builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"))).workspace.package.version
  }-git";

  src = lib.cleanSource ../.;

  cargoLock.lockFile = "${src}/Cargo.lock";

  buildNoDefaultFeatures = true;
  buildFeatures =
    lib.optional jpegSupport "jpeg"
    ++ lib.optional pnmSupport "pnm"
    ++ lib.optional qoiSupport "qoi"
    ++ lib.optional webpSupport "webp"
    ++ lib.optional avifSupport "avif"
    ++ lib.optional jxlSupport "jxl"
    ++ lib.optional clipboardSupport "clipboard"
    ++ lib.optional colorPickerSupport "color_picker"
    ++ lib.optional completionsSupport "completions"
    ++ lib.optional loggerSupport "logger"
    ++ lib.optional notificationsSupport "notifications"
    ++ lib.optional selectorSupport "selector";

  nativeBuildInputs = [
    pkg-config
    installShellFiles
    scdoc
  ];

  buildInputs =
    [wayland]
    ++ lib.optional (selectorSupport || colorPickerSupport) pango;

  postInstall =
    ''
      installManPage docs/wayshot.1.gz docs/wayshot.5.gz docs/wayshot.7.gz
    ''
    + lib.optionalString (completionsSupport && stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
      installShellCompletion --cmd wayshot \
        --bash <($out/bin/wayshot --completions bash) \
        --fish <($out/bin/wayshot --completions fish) \
        --zsh <($out/bin/wayshot --completions zsh) \
        --nushell <($out/bin/wayshot --completions nushell)
    '';

  meta = {
    description = "Screenshot crate for wlroots based compositors implementing the zwlr_screencopy_v1 protocol.";
    homepage = "https://crates.io/crates/wayshot";
    license = lib.licenses.gpl3Only;
    mainProgram = "wayshot";
    platforms = lib.platforms.linux;
  };
}
