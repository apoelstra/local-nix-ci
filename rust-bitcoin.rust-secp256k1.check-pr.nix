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
  mtxName = self: "${self.src.shortId}-${self.workspace}-${self.rustc.name}-${builtins.baseNameOf self.lockFile}-${builtins.concatStringsSep "," (map (name: builtins.substring 0 8 name) self.features)}";
  isTip = src: src == builtins.head gitCommits;

  libsecpSrc = fetchGit {
    url = "https://github.com/bitcoin-core/secp256k1/";
    ref = "master";
  };

  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      # Main project
      {
        projectName = jsonConfig.repoName;
        inherit isTip srcName mtxName prNum;

        workspace = "secp256k1";
        features = [
          [ ]
          [ "std" ]
          [ "alloc" ]
          [ "bitcoin-hashes" ]
          [ "bitcoin-hashes-std" ]
          [ "rand" ]
          [ "rand-std" ]
          [ "recovery" ]
          [ "lowmemory" ]
          [ "serde" ]
          [ "global-context" ]
          [ "global-context-less-secure" ]
          [ "global-context" "global-context-less-secure" ]
          [ "std" "bitcoin-hashes" "bitcoin-hashes-std" "rand" "rand-std" "recovery" "lowmemory" "global-context" "global-context-less-secure" "serde" ]
          [ "bitcoin-hashes" "rand" "recovery" "lowmemory" "global-context" "global-context-less-secure" "serde" ]
        ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }


      # secp256k1-sys
      {
        projectName = jsonConfig.repoName;
        inherit isTip srcName mtxName prNum;

        workspace = "secp256k1-sys";
        features = [
          [ ]
          [ "lowmemory" ]
          [ "recovery" ]
          [ "alloc" ]
          [ "std" ]
          [ "std" "lowmemory" "recovery" ]
        ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }
    ];

    singleCheckMemo = utils.crate2nixSingleCheckMemo;

    singleCheckDrv =
      { projectName
      , prNum
      , isTip
      , workspace
      , features
      , rustc
      , lockFile
      , src
      , srcName
      , mtxName
      ,
      }:
      nixes:
        with pkgs;
        let
          pkgs = import <nixpkgs> {
            overlays = [ (self: super: { inherit rustc; }) ];
          };
          drv = nixes.called.workspaceMembers.${workspace}.build.override {
            inherit features;
            runTests = true;
            testPreRun = ''
              ${rustc}/bin/rustc -V
              ${rustc}/bin/cargo -V
              echo "Source: ${builtins.toJSON src}"
              echo "Features: ${builtins.toJSON features}"
            '';
            testPostRun =
              if isTip src && isNightly rustc
              then ''
                export PATH=$PATH:${rustc}/bin:${gcc}/bin
                export CARGO_TARGET_DIR=$PWD/target
                export CARGO_HOME=${nixes.generated}/cargo
                pushd ${nixes.generated}/crate
                cargo clippy --locked -- -D warnings
                #cargo fmt --all -- --check
                popd
              ''
              else "";
          };
        in
        drv.overrideDerivation (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = projectName;
          checkPrPrNum = prNum;
          checkPrWorkspace = workspace;
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
        isTip = _: true;
        src = rec {
          src = builtins.fetchGit {
            allRefs = true;
            url = jsonConfig.gitDir;
            rev = prNum;
          };
          name = builtins.toString prNum;
          shortId = name;
          commitId = builtins.substring 0 8 name;
        };
      })
      checkData.argsMatrices;
  });
}
