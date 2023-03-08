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
  tools-nix = pkgs.callPackage utils.tools-nix-path {};
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
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${builtins.baseNameOf self.lockFile}-${builtins.concatStringsSep "," self.features}";
  isTip = src: src == builtins.head gitCommits;
  
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      # Main project
      {
        projectName = "simplicity";
        inherit isTip srcName mtxName prNum;
        features = [ [] ["bitcoin"] ["elements"] ["bitcoin" "elements"] ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }


      # simplicity-sys
      {
        projectName = "simplicity-sys";
        inherit isTip srcName mtxName prNum;

        features = [ [] ["test-utils"] ];
        rustc = allRustcs;
        src = map (commit: commit // {
          src = "${commit.src}/simplicity-sys";
          shortId = "simplicity-sys-${commit.shortId}";
        }) gitCommits;
        # FIXME avoid hardcoding this
        lockFile = /home/apoelstra/code/BlockstreamResearch/rust-simplicity/Cargo.simplicity-sys.lock;
      }
    ];
  
    singleCheckMemo = utils.crate2nixSingleCheckMemo;

    singleCheckDrv = {
      projectName,
      prNum,
      features,
      rustc,
      lockFile,
      src,
      isTip,
      srcName,
      mtxName,
    }:
    nixes:
    with pkgs;
    let
      pkgs = import <nixpkgs> {
        overlays = [ (self: super: { inherit rustc; }) ];
      };
    in nixes.called.rootCrate.build.override {
      inherit features;
      runTests = true;
      testPreRun = ''
        ${rustc}/bin/rustc -V
        ${rustc}/bin/cargo -V
        echo "Source: ${builtins.toJSON src}"
        echo "Features: ${builtins.toJSON features}"
      '';
      testPostRun = if isTip src
      then ''
        export PATH=$PATH:${rustc}/bin
        cargo fmt --check
        cargo clippy
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
          url = jsonConfig.gitDir;
          ref = prNum;
        };
        name = builtins.toString prNum;
      };
    }) checkData.argsMatrices;
  });
}
