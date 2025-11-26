let
  utils = import ./andrew-utils.nix { };
  lib = utils.nixpkgs.lib;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverrideWithPrev = prev: {
      runClippy = { src, features, rustc, isMainWorkspace, isMainLockFile, msrv, ... } @ args: (prev.runClippy args) && msrv >= "1.63.0";
  };
}
