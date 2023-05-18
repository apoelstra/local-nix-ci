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
  jsonConfig = lib.trivial.importJSON jsonConfigFile;
  allRustcs = [
    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.48.0".default
  ];
  gitCommits = utils.githubPrSrcs {
    # This must be a .git directory, not a URL or anything, since githubPrCommits
    # well set the GIT_DIR env variable to it before calling git commands. The
    # intention is for this to be run locally.
    gitDir = /. + jsonConfig.gitDir;
    gitUrl = jsonConfig.gitUrl;
    inherit prNum;
  };
  lockFileName = attrs: builtins.unsafeDiscardStringContext (builtins.baseNameOf (attrs.lockFileFn attrs.src));
  srcName = self: self.src.commitId;
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${self.workspace}-${lockFileName self}-${builtins.concatStringsSep "," self.features}";
  lockFileFn = [
    (src: "${src.src}/Cargo-minimal.lock")
    (src: "${src.src}/Cargo-recent.lock")
  ];
  lockFileFn1 = map (x: (src: /. + x)) jsonConfig.lockFiles;
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName lockFileFn;
        isTip = false;

        workspace = "bitcoin";
        features = [
          [ "default" ]
          [ "std" "rand-std" ]
          [ "std" "bitcoinconsenus-std" ]
          [ "std" "rand-std" "bitcoinconsenus-std" ]
          [ "default" "serde" "rand" ]
          [ "default" "base64" "serde" "rand" "rand-std" "secp-lowmemory" "bitcoinconsensus-std" ]

          [ "no-std" ]
          [ "no-std" "base64" ]
          [ "no-std" "rand" ]
          [ "no-std" "serde" ]
          [ "no-std" "secp-lowmemory" ]
          [ "no-std" "secp-recovery" ]
          [ "no-std" "bitcoinconsenus" ]
          [ "no-std" "secp-recovery" "secp-lowmemory" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }
      # bitcoin, no-std (does not work on 1.48)
      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName lockFileFn;
        isTip = false;

        workspace = "bitcoin";
        features = [
        ];
        rustc = [
          (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
          pkgs.rust-bin.stable.latest.default
          pkgs.rust-bin.beta.latest.default
          pkgs.rust-bin.stable."1.50.0".default
        ];
        src = gitCommits;
      }

      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName lockFileFn;
        isTip = false;

        workspace = "bitcoin_hashes";
        features = [
          [ ]
          [ "default" ]
          [ "alloc" ]
          [ "serde" ]
          [ "std" "schemars" ] # Note schemars does NOT work with nostd
          [ "std" "serde" ]
          [ "std" "serde-std" ]
          [ "serde-std" ]
          [ "std" "serde-std" "alloc" ]
          [ "std" "serde" "serde-std" "alloc" "schemars" ]
        ];
        rustc = allRustcs;
        src = builtins.head gitCommits;
      }

/*
      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName lockFileFn;
        isTip = false;

        workspace = "bitcoin-private";
        features = [
          [ ]
          [ "alloc" ]
          [ "std" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }
*/

      # Only tip
      {
        projectName = jsonConfig.repoName;
        inherit srcName prNum lockFileFn;

        isTip = true;

#        workspace = [ "bitcoin" "bitcoin-private" "bitcoin_hashes" ];
        workspace = [ "bitcoin" "bitcoin_hashes" ];
        features = [ [ "default" ] ];
        rustc = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
        src = builtins.head gitCommits;

        mtxName = self: (mtxName self) + "-tip";
      }
    ];

    singleCheckMemo = attrs:
      let tweakAttrs = attrs // { lockFile = attrs.lockFileFn attrs.src; };
      in utils.crate2nixSingleCheckMemo tweakAttrs;

    singleCheckDrv =
      { projectName
      , prNum
      , isTip
      , workspace
      , features
      , rustc
      , lockFileFn
      , src
      , srcName
      , mtxName
      ,
      }:
      nixes:
      let
        pkgs = import <nixpkgs> {
          overlays = [ (self: super: { inherit rustc; }) ];
        };
      in
      nixes.called.workspaceMembers.${workspace}.build.override {
        inherit features;
        runTests = true;
        testPreRun = ''
          ${rustc}/bin/rustc -V
          ${rustc}/bin/cargo -V
          echo "Tip: ${builtins.toString isTip}"
          echo "PR: ${prNum}"
          echo "Commit: ${src.commitId}"
          echo "Workspace ${workspace} / Features: ${builtins.toJSON features}"
        '';
        # cargo clippy runs on all workspaces at once, so rather than doing it
        # repeatedly for every workspace, just choose one ("bitcoin") and only
        # run it there..
        testPostRun =
          if workspace == "bitcoin" && isTip
          then ''
            export PATH=$PATH:${pkgs.gcc}/bin:${rustc}/bin

            export CARGO_TARGET_DIR=$PWD/target
            pushd ${nixes.generated}/crate
            CARGO_HOME=../cargo cargo clippy --locked #  -- -D warnings
            popd
          ''
          else "";
      };
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr (checkData // rec {
    argsMatrices = map
      (argsMtx: argsMtx // {
        src = {
          src = builtins.fetchGit {
            allRefs = true;
            url = jsonConfig.gitDir;
            rev = singleRev;
          };
          name = builtins.toString prNum;
          shortId = builtins.toString prNum;
          commitId = builtins.toString prNum;
        };
      })
      checkData.argsMatrices;
  });
}
