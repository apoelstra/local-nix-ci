{ pkgs ? import <nixpkgs> { }
, jsonConfigFile
, inlineJsonConfig ? null
, inlineCommitList ? []
, prNum
}:
let
  overridePkgs = import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  };
  oldFeatures = { rustc, ... }: [ [ "std" ] [ "std" "compiler" ]  [ "std" "compiler" "trace" ] ]
    ++ (if builtins.isNull (builtins.match "1.41" rustc.version) then [ [ "no-std" ] [ "no-std" "compiler" "trace" ] ] else [])
    ++ (if utils.rustcIsNightly rustc then [ [ "std" "unstable" "compiler" ] [ "no-std" "unstable" "compiler" ] ] else []);

  utils = import ./andrew-utils.nix { };
  jsonConfig = if builtins.isNull inlineJsonConfig
    then utils.parseRustConfig { inherit jsonConfigFile prNum; }
    else inlineJsonConfig // {
        gitCommits = map utils.srcFromCommit inlineCommitList;
    };
  fullMatrix = {
    inherit prNum;
    inherit (utils.standardRustMatrixFns jsonConfig)
      projectName src lockFile srcName mtxName
      isMainLockFile isMainWorkspace mainCargoToml workspace cargoToml
      rustc # override for backports 10.x and below
      msrv # override for backports 10.x and below
      # features # Must be overridden if there are any exceptional feature combinations
      runClippy
      runFmt
      runDocs;

    features = utils.featuresForSrc { needsNoStd = true; };
# for old versions pr 11.x
#    features = oldFeatures;
    # For 10.x and below
#    rustc = [ overridePkgs.rust-bin.stable.latest.default overridePkgs.rust-bin.stable."1.41.1".default overridePkgs.rust-bin.stable."1.47.0".default overridePkgs.rust-bin.beta.latest.default overridePkgs.rust-bin.nightly."2023-06-01".default ];
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

