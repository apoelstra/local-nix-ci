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
      isMainLockFile isMainWorkspace mainCargoToml cargoToml
      workspace
      features # Must be overridden if there are any exceptional feature combinations
      runClippy
      runDocs;

    runFmt = false; # we don't do rustfmt here
  };

  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";
    argsMatrix = fullMatrix;
    singleCheckDrv = utils.crate2nixSingleCheckDrv;
    memoGeneratedCargoNix = utils.crate2nixMemoGeneratedCargoNix;
    memoCalledCargoNix = utils.crate2nixMemoCalledCargoNix;
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr checkData;
}
