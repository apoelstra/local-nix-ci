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
      features # Must be overridden if there are any exceptional feature combinations
      runClippy
      runFmt
      runDocs;

    simplicityRevFile = { src, ... }: builtins.elemAt (builtins.split "\n"
      (builtins.readFile "${src.src}/simplicity-sys/depend/simplicity-HEAD-revision.txt"))
      2;
    simplicitySrc = { simplicityRevFile, ... }: builtins.fetchGit {
      allRefs = true;
      url = "https://github.com/BlockstreamResearch/simplicity/";
      rev = simplicityRevFile;
    };

    extraTestPostRun = { workspace, simplicitySrc, ... }:
      if workspace == "."
        then ''
          # Check whether jets are consistent with upstream
          ${(import "${simplicitySrc}/default.nix" {}).haskell}/bin/GenRustJets
          set -x
          diff jets_ffi.rs ./simplicity-sys/src/c_jets/jets_ffi.rs
          diff jets_wrapper.rs ./simplicity-sys/src/c_jets/jets_wrapper.rs
          diff core.rs ./src/jet/init/core.rs
          diff bitcoin.rs ./src/jet/init/bitcoin.rs
          diff elements.rs ./src/jet/init/elements.rs
          rm jets_ffi.rs
          rm jets_wrapper.rs
          rm core.rs
          rm bitcoin.rs
          rm elements.rs
        ''
      else if workspace == "simplicity-sys"
        then ''
          # Check whether C code is consistent with upstream, explicitly excluding
          # simplicity_alloc.h which we need to rewrite for the Rust bindings.
          set -x
          diff -r -x simplicity_alloc.h ${simplicitySrc}/C depend/simplicity/
        ''
      else ''
        # This code predates the 2024-05 cleanup of the local CI script.
        # As near as I can tell it was never called, since it was in the
        # `else` clause of a check of `workspace` that could never be hit.
        #

        #export CARGO_HOME=$NIXES_GENERATED_DIR/cargo
        #pushd "$NIXES_GENERATED_DIR/crate/jets-bench"
        #  cargo clippy --locked -- -D warnings
        #  cargo test --locked
        #  cargo criterion --locked --no-run
        #popd
     '';
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
