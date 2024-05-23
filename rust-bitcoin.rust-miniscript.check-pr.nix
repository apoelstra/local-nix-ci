{ pkgs ? import <nixpkgs> { }
, jsonConfigFile
, prNum
}:
let
  utils = import ./andrew-utils.nix { };
  jsonConfig = utils.parseRustConfig { inherit jsonConfigFile prNum; };
  fullMatrix = {
    inherit prNum;
    inherit (utils.standardRustMatrixFns jsonConfig)
      projectName src rustc lockFile srcName mtxName
      isMainLockFile isMainWorkspace mainCargoToml workspace cargoToml
      # features # Must be overridden if there are any exceptional feature combinations
      runClippy
      runFmt
      runDocs;

    features = utils.featuresForSrc { needsNoStd = true; };
  };

  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";
    argsMatrix = fullMatrix;
    singleCheckMemo = utils.crate2nixSingleCheckMemo;
    singleCheckDrv = utils.crate2nixSingleCheckDrv;
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr checkData;
}

