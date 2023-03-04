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
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${builtins.baseNameOf self.lockFile}-${builtins.concatStringsSep "," self.features}";
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      {
        projectName = jsonConfig.repoName;
        inherit srcName mtxName prNum;

        isTip = false;

        features = [
          []
          ["default"]
          ["strict"]
          ["default" "strict"]
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

        features = [ [ "default" "strict" ] ];
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
      features,
      rustc,
      lockFile,
      src,
      srcName,
      mtxName,
    }:
    calledCargoNix:
    with pkgs;
    let
      pkgs = import <nixpkgs> {
        overlays = [ (self: super: { inherit rustc; }) ];
      };
    in calledCargoNix.rootCrate.build.override {
      inherit features;
      runTests = true;
      testPreRun = ''
        ${rustc}/bin/rustc -V
        ${rustc}/bin/cargo -V
        echo "Features: ${builtins.toJSON features}"
      '';
      testPostRun = if isTip
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
  checkHead = utils.checkPr (checkData // {
    argsMatrices = map (argsMtx: argsMtx // {
      src = rec {
        src = builtins.fetchGit {
          url = jsonConfig.gitDir;
          ref = prNum;
        };
        name = builtins.toString prNum;
        shortId = name;
        commitId = shortId;
      };
    }) checkData.argsMatrices;
  });
}
