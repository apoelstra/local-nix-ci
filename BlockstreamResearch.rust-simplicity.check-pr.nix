{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, jsonConfigFile
, prNum
}:
let
  utils = import ./andrew-utils.nix { };
  tools-nix = pkgs.callPackage utils.tools-nix-path { };
  jsonConfig = lib.trivial.importJSON jsonConfigFile;
  allRustcs = [
    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.41.0".default
  ];
  isNightly = rustc: rustc == builtins.head allRustcs;
  gitCommits = utils.githubPrSrcs {
    # This must be a .git directory, not a URL or anything, since githubPrCommits
    # well set the GIT_DIR env variable to it before calling git commands. The
    # intention is for this to be run locally.
    gitDir = /. + jsonConfig.gitDir;
    gitUrl = jsonConfig.gitUrl;
    inherit prNum;
  };
  srcName = self: self.src.commitId;
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${builtins.baseNameOf self.lockFile}-${builtins.concatStringsSep "," self.features}";
  isTip = src: src == builtins.head gitCommits;

  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      # Main project
      {
        projectName = "simplicity";
        inherit isTip srcName mtxName prNum;
        features = [ [ ] [ "bitcoin" ] [ "elements" ] [ "bitcoin" "elements" ] ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }


      # simplicity-sys
      {
        projectName = "simplicity-sys";
        inherit isTip srcName mtxName prNum;

        features = [ [ ] [ "test-utils" ] ];
        rustc = allRustcs;
        src = map
          (commit: commit // {
            src = "${commit.src}/simplicity-sys";
            shortId = "simplicity-sys-${commit.shortId}";
          })
          gitCommits;
        # FIXME avoid hardcoding this
        lockFile = /home/apoelstra/code/BlockstreamResearch/rust-simplicity/Cargo.simplicity-sys.lock;
      }
    ];

    singleCheckMemo = utils.crate2nixSingleCheckMemo;

    singleCheckDrv =
      { projectName
      , prNum
      , features
      , rustc
      , lockFile
      , src
      , isTip
      , srcName
      , mtxName
      ,
      }:
      nixes:
        with pkgs;
        let
          simplicityRevFile = if projectName == "simplicity"
          then builtins.readFile "${src.src}/simplicity-sys/depend/simplicity-HEAD-revision.txt"
          else builtins.readFile "${src.src}/depend/simplicity-HEAD-revision.txt";
          simplicitySrc = builtins.fetchGit {
            allRefs = true;
            url = "https://github.com/BlockstreamResearch/simplicity/";
            rev = builtins.elemAt (builtins.split "\n" simplicityRevFile) 2;
          };
          pkgs = import <nixpkgs> {
            overlays = [ (self: super: { inherit rustc; }) ];
          };
          drv = nixes.called.rootCrate.build.override {
            inherit features;
            runTests = true;
            testPreRun = ''
              ${rustc}/bin/rustc -V
              ${rustc}/bin/cargo -V
              echo "Source: ${builtins.toJSON src}"
              echo "Features: ${builtins.toJSON features}"
            '';
            testPostRun = lib.optionalString (isTip src && isNightly rustc) (
              if projectName == "simplicity"
              then ''
                # Check whether jets are consistent with upstream
                ${(import "${simplicitySrc}/default.nix" {}).haskell}/bin/GenRustJets
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
              else ''
                # Check whether C code is consistent with upstream
                diff -r ${simplicitySrc}/C depend/simplicity/
              '' + ''
                export PATH=$PATH:${rustc}/bin:${gcc}/bin
                export CARGO_TARGET_DIR=$PWD/target
                export CARGO_HOME=${nixes.generated}/cargo
                pushd ${nixes.generated}/crate
                cargo clippy --locked # -- -D warnings # FIXME re-enable warnings
                cargo fmt --all -- --check
                popd
              '');
          };
        in
        drv.overrideDerivation (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = projectName;
          checkPrPrNum = prNum;
          checkPrRustc = rustc;
          checkPrFeatures = builtins.toJSON features;
          checkPrSrc = builtins.toJSON src;
        });
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr (checkData // rec {
    argsMatrices = map
      (argsMtx: argsMtx // rec {
        src =
          let
            isSys = argsMtx.projectName != "simplicity";
            gitSrc = builtins.fetchGit {
              allRefs = true;
              url = jsonConfig.gitDir;
              rev = prNum;
            };
          in rec {
              src = if isSys
              then "${gitSrc}/simplicity-sys"
              else gitSrc;
              commitId = builtins.toString prNum;
              shortId = "${builtins.substring 0 8 commitId}${lib.optionalString isSys "-sys"}";
            };
        isTip = x: true;
      })
      checkData.argsMatrices;
  });
}
