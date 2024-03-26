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
#    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    pkgs.rust-bin.nightly."2024-03-05".default
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.56.1".default
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
  lockFileName = attrs: builtins.unsafeDiscardStringContext (builtins.baseNameOf (attrs.lockFileFn attrs.src));
  srcName = self: self.src.commitId;
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${self.workspace}-${lockFileName self}-${builtins.concatStringsSep "," self.features}";
  lockFileFn = [
    (src: "${src.src}/Cargo-minimal.lock")
    (src: "${src.src}/Cargo-recent.lock")
  ];
  lockFileFn1 = map (x: (src: /. + x)) jsonConfig.lockFiles;
  fullMatrix = {
    projectName = jsonConfig.repoName;
    inherit prNum srcName mtxName lockFileFn;
    isTip = false;

    workspace = "bitcoin";
    features = [
      [ "default" ]
      [ "all-stable-features" ]
      [ "std" "rand-std" ]
      [ "std" "bitcoinconsenus-std" ]
      [ "std" "rand-std" "bitcoinconsenus-std" ]
      [ "default" "serde" "rand" ]
      [ "default" "base64" "serde" "rand" "rand-std" "secp-lowmemory" "bitcoinconsensus-std" ]

      [ "unstable" ]
      [ "unstable" "base64" ]
      [ "unstable" "rand" ]
      [ "unstable" "serde" ]
      [ "unstable" "secp-lowmemory" ]
      [ "unstable" "secp-recovery" ]
      [ "unstable" "bitcoinconsenus" ]
      [ "unstable" "secp-recovery" "secp-lowmemory" ]
    ];
    rustc = allRustcs;
    src = gitCommits;
  };
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      (fullMatrix // {
        isTip = false;
        rustc = pkgs.rust-bin.stable.latest.default;
      })
      (fullMatrix // {
        isTip = true;
        rustc = allRustcs;
        src = builtins.head gitCommits;
      })

      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName lockFileFn;
        isTip = false;

        workspace = "base58ck";
        features = [
          [ ]
          [ "default" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }

      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName lockFileFn;
        isTip = false;

        workspace = "bitcoin-units";
        features = [
          [ ]
          [ "alloc" ]
          [ "default" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }

      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName lockFileFn;
        isTip = false;

        workspace = "bitcoin-io";
        features = [
          [ ]
          [ "default" ]
#          [ "alloc" ]
        ];
        rustc = allRustcs;
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
        src = gitCommits;
      }

      {
        projectName = jsonConfig.repoName;
        inherit prNum srcName mtxName lockFileFn;
        isTip = false;

        workspace = "bitcoin-internals";
        features = [
          [ ]
          [ "alloc" ]
          [ "std" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }

      # Only tip
      {
        projectName = jsonConfig.repoName;
        inherit srcName prNum lockFileFn;

        isTip = true;

        workspace = [ "base58ck" "bitcoin" "bitcoin-internals" "bitcoin-io" "bitcoin-units" "bitcoin_hashes" ];
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
          if workspace == "bitcoin" && isNightly rustc && isTip
          then ''
            set -x
            pwd
            export PATH=$PATH:${pkgs.gcc}/bin:${rustc}/bin

            export CARGO_TARGET_DIR=$PWD/target
            pushd ${nixes.generated}/crate
            export CARGO_HOME=../cargo

            # Nightly clippy
            cargo clippy --all-features --all-targets --locked -- -D warnings
            # Do nightly "broken links" check
            ls /build/ || true
            ls /build/target || true
            ls /build/target/doc || true
            ls /build/target/doc/bitcoin_hashes || true
            #export RUSTDOCFLAGS="--cfg docsrs -D warnings -D rustdoc::broken-intra-doc-links"
            cargo doc -j1 --all-features
            #ls /build/ || true
            #ls /build/target || true
            #ls /build/target/doc || true
            #ls /build/target/doc/bitcoin_hashes || true
            # Do non-docsrs check that our docs are feature-gated correctly.
            export RUSTDOCFLAGS="-D warnings"
            cargo doc -j1 --all-features
            ls /build/ || true
            ls /build/target || true
            ls /build/target/doc || true
            ls /build/target/doc/bitcoin_hashes || true
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
