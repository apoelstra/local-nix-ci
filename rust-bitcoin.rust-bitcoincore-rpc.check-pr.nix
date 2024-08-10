{ pkgs ? import <nixpkgs> { }
, jsonConfigFile
, prNum
}:
let
  utils = import ./andrew-utils.nix { };
  jsonConfig = utils.parseRustConfig { inherit jsonConfigFile prNum; };
  fullMatrix = {
    # Much of this was disabled 2024-06-10
    inherit prNum;
    inherit (utils.standardRustMatrixFns jsonConfig)
      projectName src rustc lockFile srcName mtxName
      isMainLockFile isMainWorkspace mainCargoToml workspace cargoToml
      features; # Must be overridden if there are any exceptional feature combinations
      #runClippy # doesn't work in rust-bitcoincore-rpc
      #runFmt; # doesn't work in rust-bitcoincore-rpc
      #runDocs; # doesn't work in rust-bitcoincore-rpc
    runClippy = false;
    runFmt = false;
    runDocs = false;

    extraTestPostRun = { workspace, ... }: if workspace == "integration_test"
      then ''
        export PATH=${pkgs.psmisc}/bin:${pkgs.valgrind}/bin:$PATH # for killall
        sed -i 's/cargo run/valgrind .\/target\/debug\/integration_test/' run.sh
        sed -i 's/bitcoind/"$BITCOIND_EXE"/' run.sh
        # Disable the integration test. Seems not to work even with bitcoin as far back as 0.21.
        # Gives "wallet verification failed" errors which are not transient
        # or related to a Nix environment. They are just indicative of the tests
        # having rotted.
        #./run.sh
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
