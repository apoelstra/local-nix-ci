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
  checkData = rec {
    projectName = jsonConfig.repoName;
    inherit prNum;
    argsMatrices = [
      {
        workspace = "bitcoincore-rpc-json";
        features = [ [ ] ];
        rustc = allRustcs;
        overrideLockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }

      {
        workspace = "bitcoincore-rpc";
        features = [ [ ] ];
        rustc = allRustcs;
        overrideLockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }

      {
        workspace = "integration_test";
        features = [ [ ] ];
        rustc = allRustcs;
        overrideLockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }
    ];

    checkSingleCommit =
      { src
      , workspace
      , overrideLockFile
      , features ? [ "default" ]
      , rustc ? pkgs.rust-bin.stable.latest.default
      }:
        with pkgs;
        let
          pkgs = import <nixpkgs> {
            overlays = [ (self: super: { inherit rustc; }) ];
          };
          generated = tools-nix.generatedCargoNix {
            name = "${projectName}-generated-cargo-nix-${builtins.toString prNum}-${src.shortId}";
            src = src.src;
            inherit overrideLockFile;
          };
          called = pkgs.callPackage "${generated}/default.nix" { };
        in
        builtins.trace (called.workspaceMembers) called.workspaceMembers.${workspace}.build.override {
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
  checkHead = utils.checkPr (checkData // rec {
    argsMatrices = map
      (argsMtx: argsMtx // {
        src = {
          src = builtins.fetchGit {
            url = jsonConfig.gitDir;
            ref = prNum;
          };
          name = builtins.toString prNum;
        };
      })
      checkData.argsMatrices;
  });
}
