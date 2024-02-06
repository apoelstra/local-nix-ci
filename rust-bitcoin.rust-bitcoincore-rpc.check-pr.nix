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
#    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
#    pkgs.rust-bin.stable.latest.default
#    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.56.1".default
  ];
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
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";
    argsMatrices = [
      {
        inherit srcName mtxName prNum;
        projectName = jsonConfig.repoName;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;

        workspace = "bitcoincore-rpc-json";
        features = [ [ ] ];
        rustc = allRustcs;
        src = gitCommits;
      }

      {
        inherit srcName mtxName prNum;
        projectName = jsonConfig.repoName;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;

        workspace = "bitcoincore-rpc";
        features = [ [ ] ];
        rustc = allRustcs;
        src = gitCommits;
      }

      {
        inherit srcName mtxName prNum;
        projectName = jsonConfig.repoName;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;

        workspace = "integration_test";
        features = [ [ ] ];
        rustc = allRustcs;
        src = gitCommits;
      }
    ];

    singleCheckMemo = utils.crate2nixSingleCheckMemo;

    singleCheckDrv =
      { projectName
      , prNum
      , src
      , srcName
      , mtxName
      , workspace
      , lockFile
      , features
      , rustc
      }:
      nixes:
        let
          drv = nixes.called.workspaceMembers.${workspace}.build.override {
            inherit features;
            runTests = true;
            testPreRun = ''
              ${rustc}/bin/rustc -V
              ${rustc}/bin/cargo -V
              echo "PR: ${prNum}"
              echo "Commit: ${src.commitId}"
              echo "Features: ${builtins.toJSON features}"
            '';
            testPostRun = ''
              export PATH=$PATH:${rustc}/bin:${pkgs.gcc}/bin
              export CARGO_TARGET_DIR=$PWD/target
              export CARGO_HOME=${nixes.generated}/cargo
              pushd ${nixes.generated}/crate
              cargo clippy --locked -- -D warnings
              popd
            '';
          };
          fuzzTargets = map
            (bin: bin.name)
            (lib.trivial.importTOML "${src.src}/fuzz/Cargo.toml").bin;
          fuzzDrv = utils.cargoFuzzDrv {
            normalDrv = drv;
            inherit projectName src lockFile nixes fuzzTargets;
          };
        in drv.overrideDerivation (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = projectName;
          checkPrPrNum = prNum;
          checkPrRustc = rustc;
          checkPrLockFile = lockFile;
          checkPrFeatures = builtins.toJSON features;
          checkPrWorkspace = workspace;
          checkPrSrc = builtins.toJSON src;
        });
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr (checkData // {
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
      })
      checkData.argsMatrices;
  });
}
