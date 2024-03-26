{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, jsonConfigFile
, prNum
# Only used by checkHEad, not checkPr
, singleRev ? prNum
}:
let
  utils = import ./andrew-utils.nix { };
  tools-nix = pkgs.callPackage utils.tools-nix-path { };
  jsonConfig = lib.trivial.importJSON jsonConfigFile;
  allRustcs = [
    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.58.0".default
  ];
  gitCommits = utils.githubPrSrcs {
    # This must be a .git directory, not a URL or anything, since githubPrCommits
    # well set the GIT_DIR env variable to it before calling git commands. The
    # intention is for this to be run locally.
    gitDir = /. + jsonConfig.gitDir;
    gitUrl = jsonConfig.gitUrl;
    inherit prNum;
  };
  projectName = jsonConfig.repoName;
  isTip = { src, rustc }: src == builtins.head gitCommits && rustc == builtins.head allRustcs;
  lockFileName = attrs: builtins.unsafeDiscardStringContext (builtins.baseNameOf (attrs.lockFileFn attrs.src));
  srcName = self: self.src.commitId;
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${lockFileName self}-${builtins.concatStringsSep "," self.features}";
  lockFileFn = [
    (src: "${src.src}/Cargo-recent.lock")
  ];

  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      {
        inherit projectName srcName mtxName prNum isTip lockFileFn;
        workspace = "simplicity-lang";

        features = [ [ ] [ "bitcoin" ] [ "elements" ] [ "bitcoin" "elements" ] ];
        rustc = allRustcs;
        src = gitCommits;
      }

      {
        inherit projectName srcName mtxName prNum isTip lockFileFn;
        workspace = "simplicity-sys";

        features = [ [ ] [ "test-utils" ] ];
        rustc = allRustcs;
        src = gitCommits;
      }

      {
        inherit projectName srcName mtxName prNum isTip lockFileFn;

        workspace = "simplicity-fuzz";
        features = [ [] ];
        rustc = pkgs.rust-bin.stable."1.58.0".default;
        src = gitCommits;
      }
    ];

    singleCheckMemo = attrs:
      let tweakAttrs = attrs // { lockFile = attrs.lockFileFn attrs.src; };
      in utils.crate2nixSingleCheckMemo tweakAttrs;

    singleCheckDrv =
      { projectName
      , prNum
      , workspace
      , features
      , rustc
      , lockFileFn
      , src
      , isTip
      , srcName
      , mtxName
      ,
      }:
      nixes:
        with pkgs;
        let
          simplicityRevFile = builtins.split "\n"
            (builtins.readFile "${src.src}/simplicity-sys/depend/simplicity-HEAD-revision.txt");
          simplicitySrc = builtins.fetchGit {
            allRefs = true;
            url = "https://github.com/BlockstreamResearch/simplicity/";
            rev = builtins.elemAt simplicityRevFile 2;
          };
          checkJets = ''
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
          '';
          checkVendoredC = ''
            # Check whether C code is consistent with upstream, explicitly excluding
            # simplicity_alloc.h which we need to rewrite for the Rust bindings.
            set -x
            diff -r -x simplicity_alloc.h ${simplicitySrc}/C depend/simplicity/
          '';

          nonFuzzDrv = nixes.called.workspaceMembers.${workspace}.build.override {
            inherit features;
            runTests = true;
            testPreRun = ''
              ${rustc}/bin/rustc -V
              ${rustc}/bin/cargo -V
              echo "Source: ${builtins.toJSON src}"
              echo "Features: ${builtins.toJSON features}"
            '';
            testPostRun = lib.optionalString (isTip { inherit src rustc; }) (
              if workspace == "simplicity-lang"
              then checkJets
              else if workspace == "simplicity-sys"
              then checkVendoredC
              else "" +
              ''
                export PATH=$PATH:${rustc}/bin:${gcc}/bin:${pkgs.cargo-criterion}/bin
                export CARGO_TARGET_DIR=$PWD/target
                export CARGO_HOME=${nixes.generated}/cargo
                pushd ${nixes.generated}/crate
                cargo clippy --locked #-- -D warnings
                cargo fmt --all -- --check
                pushd jets-bench
                  cargo clippy --locked #-- -D warnings
                  cargo test --locked
                  cargo criterion --locked --no-run
                popd
              '');
          };
          fuzzTargets = map
            (bin: bin.name)
            (lib.trivial.importTOML "${src.src}/fuzz/Cargo.toml").bin;
          fuzzDrv = utils.cargoFuzzDrv {
            normalDrv = nonFuzzDrv;
            lockFile = lockFileFn src;
            inherit projectName src nixes fuzzTargets;
          };
        in
        (if workspace == "simplicity-fuzz"
        then fuzzDrv
        else nonFuzzDrv).overrideAttrs (drv: {
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
      (argsMtx: argsMtx // {
        src = rec {
          src = builtins.fetchGit {
            allRefs = true;
            url = jsonConfig.gitDir;
            rev = singleRev;
          };
          name = builtins.toString prNum;
          shortId = name;
          commitId = shortId;
        };
        isTip = { src, rustc }: rustc == builtins.head allRustcs;
      })
      checkData.argsMatrices;
  });
}
