{
  pkgs ? import <nixpkgs> {
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
  utils = import ./andrew-utils.nix {};
  jsonConfig = lib.trivial.importJSON jsonConfigFile;
  allRustcs = [
    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.41.0".default
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
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${self.workspace}-${builtins.baseNameOf self.lockFile}-${builtins.concatStringsSep "," self.features}";
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName;
        isTip = false;

        workspace = "bitcoin";
        features = [
          [ "default" ]
          [ "std" "rand-std" ]
          [ "std" "bitcoinconsenus-std" ]
          [ "std" "rand-std" "bitcoinconsenus-std" ]
          [ "default" "serde" "rand" ]
          [ "default" "base64" "serde" "rand" "rand-std" "secp-lowmemory" "bitcoinconsensus-std" ]
        ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }
      # bitcoin, no-std (does not work on 1.41)
      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName;
        isTip = false;

        workspace = "bitcoin";
        features = [
          [ "no-std" ]
          [ "no-std" "base64" ]
          [ "no-std" "rand" ]
          [ "no-std" "serde" ]
          [ "no-std" "secp-lowmemory" ]
          [ "no-std" "secp-recovery" ]
          [ "no-std" "bitcoinconsenus" ]
          [ "no-std" "secp-recovery" "secp-lowmemory" ]
        ];
        rustc = [
          (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
           pkgs.rust-bin.stable.latest.default
           pkgs.rust-bin.beta.latest.default
           pkgs.rust-bin.stable."1.50.0".default
        ];
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }

      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName;
        isTip = false;

        workspace = "bitcoin_hashes";
        features = [
          [ ]
          [ "default" ]
          [ "alloc" ]
          [ "serde" ]
          [ "std" "schemars" ]  # Note schemars does NOT work with nostd
          [ "std" "serde" ]
          [ "std" "serde-std" ]
          [ "serde-std" ]
          [ "std" "serde-std" "alloc" ]
          [ "std" "serde" "serde-std" "alloc" "schemars" ]
        ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = builtins.head gitCommits;
      }

      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName;
        isTip = false;

        workspace = "bitcoin-internals";
        features = [
          []
          [ "alloc" ]
          [ "std" ]
        ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }

      # Only tip
      {
        projectName = jsonConfig.repoName;
        inherit srcName prNum;

        isTip = true;

        workspace = [ "bitcoin" "bitcoin-internals" "bitcoin_hashes" ];
        features = [ [ "default" ] ];
        rustc = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
        lockFile = /. + builtins.head jsonConfig.lockFiles;
        src = builtins.head gitCommits;

        mtxName = self: (mtxName self) + "-tip";
      }
    ];

    singleCheckMemo = utils.crate2nixSingleCheckMemo;

    singleCheckDrv = {
      projectName,
      prNum,
      isTip,
      workspace,
      features,
      rustc,
      lockFile,
      src,
      srcName,
      mtxName,
    }:
    nixes:
    let
      pkgs = import <nixpkgs> {
        overlays = [ (self: super: { inherit rustc; }) ];
      };
    in nixes.called.workspaceMembers.${workspace}.build.override {
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
      testPostRun = if workspace == "bitcoin" && isTip
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
    argsMatrices = map (argsMtx: argsMtx // {
      src = {
        src = builtins.fetchGit {
          allRefs = true;
          url = jsonConfig.gitDir;
          rev = prNum;
        };
        name = builtins.toString prNum;
        shortId = builtins.toString prNum;
        commitId = builtins.toString prNum;
      };
    }) checkData.argsMatrices;
  });
}
