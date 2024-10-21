{ pkgs ? import <nixpkgs> {}
, utils ? import ./andrew-utils.nix {}
, fullMatrixOverride ? {}
}:
{ inlineJsonConfig
, inlineCommitList ? []
, fallbackLockFiles ? []
, prNum
}:
let
  jsonConfig = inlineJsonConfig // {
    inherit fallbackLockFiles;
    gitCommits = map utils.srcFromCommit inlineCommitList;
  };
  fullMatrix = {
    inherit prNum;
    inherit (utils.standardRustMatrixFns jsonConfig)
      projectName src msrv rustc lockFile srcName mtxName
      isMainLockFile isMainWorkspace mainCargoToml workspace cargoToml
      features # Must be overridden if there are any exceptional feature combinations
      runClippy
      runFmt
      runDocs
      releaseMode # Should override with false for slow crates!
      ;
  } // fullMatrixOverride;

  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";
    argsMatrix = fullMatrix;
    singleCheckDrv = utils.crate2nixSingleCheckDrv;
    memoGeneratedCargoNix = utils.crate2nixMemoGeneratedCargoNix;
    memoCalledCargoNix = utils.crate2nixMemoCalledCargoNix;
  };
in
  utils.checkPr checkData
