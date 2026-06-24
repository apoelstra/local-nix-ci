let
  utils = import ./andrew-utils.nix { };
  lib = utils.overlaidPkgs.lib;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverrideWithPrev = prev: {
    runDocs = false; # 2024-09-24 -- should fix, several bugs
    runFmt = false; # we don't do rustfmt here
    runClippy = false;

    # disable integration tests for now; failing on master, running forever when
    # I try bumping the elementsd version ... let's update to the main miniscript
    # version first
    workspace = { mainMajorRev, ... } @ args:
      builtins.filter (x: x != "bitcoind-tests") (prev.workspace args);
  };
}
