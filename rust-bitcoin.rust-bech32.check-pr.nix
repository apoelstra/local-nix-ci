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
      projectName src rustc msrv lockFile srcName mtxName
      isMainLockFile isMainWorkspace mainCargoToml workspace cargoToml
      runClippy
      runFmt
      runDocs;
    features = {src, cargoToml, ...}:
      builtins.trace (builtins.toJSON ((utils.standardRustMatrixFns jsonConfig).features { inherit src cargoToml; }))
       (utils.standardRustMatrixFns jsonConfig).features { inherit src cargoToml; };
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
