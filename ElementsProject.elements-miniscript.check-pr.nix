let
  utils = import ./andrew-utils.nix { };
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    runDocs = false; # 2024-09-24 -- should fix, several bugs
    runFmt = false; # we don't do rustfmt here
  };
}
