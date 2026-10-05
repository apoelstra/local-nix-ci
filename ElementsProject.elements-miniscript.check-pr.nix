let
  utils = import ./andrew-utils.nix { };
  lib = utils.overlaidPkgs.lib;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverrideWithPrev = prev: {
    runDocs = false; # 2024-09-24 -- should fix, several bugs
    runFmt = false; # we don't do rustfmt here
    runClippy = false;
  };
}
