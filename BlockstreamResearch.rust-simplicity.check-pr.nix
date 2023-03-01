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
  gitCommitDrv = import (utils.githubPrCommits {
    # This must be a .git directory, not a URL or anything, since githubPrCommits
    # well set the GIT_DIR env variable to it before calling git commands. The
    # intention is for this to be run locally.
    gitDir = /. + jsonConfig.gitDir;
    inherit prNum;
  }) {};
  gitCommits = gitCommitDrv.gitCommits;
  checkData = rec {
    projectName = jsonConfig.repoName;
    inherit prNum;
    argsMatrix = rec {
      features = [
        []
        ["bitcoin"]
        ["elements"]
        ["bitcoin" "elements"]
      ];
      rustc = [
        (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
        pkgs.rust-bin.stable.latest.default
        pkgs.rust-bin.beta.latest.default
        pkgs.rust-bin.stable."1.41.0".default
      ];
      overrideLockFile = map (x: /. + x) jsonConfig.lockFiles;
      src = map (commit: {
        src = builtins.fetchGit {
          url = jsonConfig.gitUrl;
          ref = "refs/pull/${builtins.toString prNum}/head";
          rev = commit;
        };
        name = builtins.substring 0 8 commit;
      }) gitCommits;
    };
  
    checkSingleCommit =
    { src
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
        name = src.name; #projectName + "-" + src.name;
        src = src.src;
        inherit overrideLockFile;
      };
      called = pkgs.callPackage "${generated}/default.nix" {};
    in
      called.rootCrate.build.override {
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
    argsMatrix = checkData.argsMatrix // {
      src = {
        src = builtins.fetchGit {
          url = jsonConfig.gitDir;
          ref = prNum;
        };
        name = builtins.toString prNum;
      };
    };
  });
}
