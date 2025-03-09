{
  description = "Development environment for wayshot";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
    {
      devShells.default = import ./shell.nix;
    });
}
