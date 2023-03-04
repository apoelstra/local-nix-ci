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
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [{
      projectName = jsonConfig.repoName;
      inherit prNum;

      features = [
        []
        ["default"]
        ["strict"]
        ["default" "strict"]
      ];
      rustc = allRustcs;
      overrideLockFile = map (x: /. + x) jsonConfig.lockFiles;
      src = gitCommits;

      srcName = self: self.src.shortId;
    }];

    singleCheckMemo = {
      projectName,
      prNum,
      overrideLockFile,
      src,
      ...
    }:
    let generatedCargoNix = tools-nix.generatedCargoNix {
      name = "${projectName}-generated-cargo-nix-${builtins.toString prNum}-${src.shortId}";
      src = src.src;
      inherit overrideLockFile;
    };
    in {
      name = builtins.unsafeDiscardStringContext (builtins.toString generatedCargoNix);
      value = pkgs.callPackage generatedCargoNix {};
    };

    singleCheckDrv = {
      projectName,
      prNum,
      features,
      rustc,
      overrideLockFile,
      src,
      srcName,
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
    };
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr (checkData // {
    gitCommits = [{
      src = {
        src = builtins.fetchGit {
          url = jsonConfig.gitDir;
          ref = prNum;
        };
        name = builtins.toString prNum;
        shortId = builtins.toString prNum;
      };
    }];
  });
}
