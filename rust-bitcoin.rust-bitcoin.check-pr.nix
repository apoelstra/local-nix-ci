{ pkgs ? import <nixpkgs> { }
, jsonConfigFile
, prNum
}:
let
  utils = import ./andrew-utils.nix { };
  jsonConfig = utils.parseRustConfig { inherit jsonConfigFile prNum; };
  fullMatrix = {
    inherit prNum;
    inherit (utils.standardRustMatrixFns jsonConfig) projectName src rustc lockFile srcName mtxName isTip workspace;

    features = { workspace, ... }: if workspace == "bitcoin" then [
      [ ]
      [ "default" ]
      [ "std" "rand-std" ]
      [ "std" "bitcoinconsensus-std" ]
      [ "std" "rand-std" "bitcoinconsensus-std" ]
      [ "default" "serde" "rand" ]
      [ "default" "base64" "serde" "rand" "rand-std" "secp-lowmemory" "bitcoinconsensus-std" ]
      [ "serde" "rand" ]
      [ "base64" "serde" "rand" "secp-lowmemory" "bitcoinconsensus" ]
    ]
    else if workspace == "base58" then [
      [ ]
      [ "default" ]
    ]
    else if workspace == "hashes" then [
      [ ]
      [ "default" ]
      [ "alloc" ]
      [ "serde" ]
      [ "std" "schemars" ] # Note schemars does NOT work with nostd
      [ "std" "serde" ]
      [ "std" "serde" "alloc" "schemars" ]
    ]
    else if workspace == "internals" then [
      [ ]
      [ "alloc" ]
      [ "std" ]
    ]
    else if workspace == "io" then [
      [ ]
      [ "default" ]
    ]
    else if workspace == "units" then [
      [ ]
      [ "alloc" ]
      [ "default" ]
    ]
    else if workspace == "fuzz" then [ [] ] # Fuzz is treated specially
    else builtins.abort "Unknown workspace ${workspace}!";

    # Clippy runs with --all-targets so we only need to run it on one workspace.
    runClippy = { workspace, isTip, ... }: workspace == "bitcoin" && isTip;
    runDocs = { workspace, isTip, ... }: workspace == "bitcoin" && isTip;
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
