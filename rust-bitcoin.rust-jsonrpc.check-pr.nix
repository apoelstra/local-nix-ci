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
      features # Must be overridden if there are any exceptional feature combinations
      runClippy
      runFmt
      runDocs;

    extraTestPostRun = { workspace, ... }: if workspace == "integration_test"
      then ''
        set -x
        export BITCOIND_PATH=$BITCOIND_EXE
        export PATH=${pkgs.psmisc}/bin:${pkgs.valgrind}/bin:$PATH # for killall
        sed -i 's/cargo run/valgrind .\/target\/debug\/integration_test/' run.sh
        cat run.sh
        ./run.sh
      ''
      else "";
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
