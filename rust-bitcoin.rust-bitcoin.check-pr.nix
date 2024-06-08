{ pkgs ? import <nixpkgs> { }
, jsonConfigFile
, prNum
}:
let
  utils = import ./andrew-utils.nix { };
  jsonConfig = utils.parseRustConfig { inherit jsonConfigFile prNum; };
  fullMatrix = {
    inherit prNum;
    inherit (utils.standardRustMatrixFns jsonConfig) projectName src rustc lockFile srcName mtxName isMainLockFile isMainWorkspace mainCargoToml workspace cargoToml runClippy runDocs runCheckPublicApi;

    features = { src, cargoToml, workspace, ... }:
      if workspace == "bitcoin"
      then utils.featuresForSrc { exclude = [ "actual-serde" ]; } { inherit src cargoToml; }
      # schemars does not work with nostd, so exclude it from
      # the standard list and test it separately.
      else if workspace == "hashes"
      then utils.featuresForSrc {
        include = [ [ "std" "schemars" ] ];
        exclude = [ "actual-serde" "schemars" ];
      } { inherit src cargoToml; }
      else utils.featuresForSrc {} { inherit src cargoToml; };

      runFmt = false;
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
