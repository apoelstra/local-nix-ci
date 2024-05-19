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
  lockFileName = path: builtins.unsafeDiscardStringContext (builtins.baseNameOf path);
  srcName = { src, ... }: src.commitId;
  mtxName = { src, rustc, workspace, features, lockFile, isTip, ... }: "${src.shortId}-${rustc.name}-${workspace}-${lockFileName lockFile}-${builtins.concatStringsSep "," features}${if isTip then "-tip" else ""}";
  lockFile = { src, ... }: [
    "${src.src}/Cargo-minimal.lock"
    "${src.src}/Cargo-recent.lock"
  ];
  fullMatrix = {
    projectName = jsonConfig.repoName;
    rustc = allRustcs;
    src = gitCommits;

    inherit prNum srcName mtxName lockFile;
    isTip = { rustc, src, ... }:
      rustc == builtins.head allRustcs && src.isTip;

    workspace = { src, ... }: (lib.trivial.importTOML "${src.src}/Cargo.toml").workspace.members;

    features = { workspace, ... }: if workspace == "bitcoin" then [
      [ "default" ]
      [ "std" "rand-std" ]
      [ "std" "bitcoinconsensus-std" ]
      [ "std" "rand-std" "bitcoinconsensus-std" ]
      [ "default" "serde" "rand" ]
      [ "default" "base64" "serde" "rand" "rand-std" "secp-lowmemory" "bitcoinconsensus-std" ]
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
  };
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [ fullMatrix ];

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
      let
        pkgs = import <nixpkgs> {
          overlays = [ (self: super: { inherit rustc; }) ];
        };
        workspaceToml = lib.trivial.importTOML "${src.src}/${workspace}/Cargo.toml";
        workspaceName = workspaceToml.package.name;
        allowedFeatures = builtins.attrNames workspaceToml.features
          ++ (if workspaceToml ? dependencies
            then builtins.attrNames (lib.filterAttrs (_: opt: opt ? optional && opt.optional) workspaceToml.dependencies)
            else [])
          ++ [ "default" ];
        drv = if ! lib.all (feat: lib.elem feat allowedFeatures) features
        then builtins.abort "Feature list ${builtins.toString features} had feature not in TOML ${builtins.toString allowedFeatures}"
        else nixes.called.workspaceMembers.${workspaceName}.build.override {
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
              # Check API

              popd
            ''
            else "";
        };
        fuzzTargets = map
          (bin: bin.name)
          (lib.trivial.importTOML "${src.src}/fuzz/Cargo.toml").bin;
        fuzzDrv = utils.cargoFuzzDrv {
          normalDrv = drv;
          inherit projectName src lockFile nixes fuzzTargets;
        };
      in
        if workspace == "fuzz"
        then fuzzDrv
        else drv.overrideDerivation (drv: {
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
          isTip = true;
        };
      })
      checkData.argsMatrices;
  });
}
