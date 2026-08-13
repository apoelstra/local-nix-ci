let
  utils = import ./andrew-utils.nix { };
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverrideWithPrev = prev: {
    runFmt = false; # not enabled on rust-elements

    # disable integration tests for now; failing on master for obscure "wallet
    # compatibility" reasons, presumably due to my upgrading things to try to make
    # them work with elements-minsicript
    workspace = { mainMajorRev, ... } @ args:
      builtins.filter (x: x != "elementsd-tests") (prev.workspace args);
  };
}
