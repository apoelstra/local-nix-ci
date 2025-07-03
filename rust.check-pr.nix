{ pkgs ? import <nixpkgs> {}
, utils ? import ./andrew-utils.nix {}
, fullMatrixOverride ? {}
, fullMatrixOverrideWithPrev ? prev: {}
}:
{ inlineJsonConfig
, inlineCommitList ? []
, prNum
}:
let
  jsonConfig = inlineJsonConfig // {
    gitCommits = map utils.srcFromCommit inlineCommitList;
  };
  fullMatrixPrev = {
    inherit prNum;
    inherit (utils.standardRustMatrixFns jsonConfig)
      projectName src msrv rustc cargoNix lockFile srcName mtxName
      isMainLockFile isMainWorkspace mainCargoToml mainMajorRev workspace cargoToml
      features # Must be overridden if there are any exceptional feature combinations
      runClippy
      runFmt
      runDocs
      releaseMode # Should override with false for slow crates!
      ;
  } // fullMatrixOverride;

  fullMatrix = fullMatrixPrev // (fullMatrixOverrideWithPrev fullMatrixPrev);

  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";
    argsMatrix = fullMatrix;
    singleCheckDrv = utils.crate2nixSingleCheckDrv;
    memoGeneratedCargoNix = utils.crate2nixMemoGeneratedCargoNix;
    memoCalledCargoNix = utils.crate2nixMemoCalledCargoNix;
  };
in
  utils.checkPr checkData
