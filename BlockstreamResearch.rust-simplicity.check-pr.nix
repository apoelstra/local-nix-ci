{
  pkgs ? import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, gitUrl
, prNum 
}:
let
  utils = import ./andrew-utils.nix {};
  tools-nix = pkgs.callPackage utils.tools-nix-path {};
  gitCommitDrv = import (utils.githubPrCommits {
    gitDir = ./master/.git;
    inherit prNum;
  }) {};
  gitCommits = gitCommitDrv.gitCommits;
  checkData = rec {
    projectName = builtins.baseNameOf gitUrl;
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
      overrideLockFile = [ ./Cargo.latest.lock ./Cargo.minimal.lock ];
      src = map (commit: {
        src = builtins.fetchGit {
          url = gitUrl;
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
        name = projectName + "-" + src.name;
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
          url = gitUrl;
          ref = prNum;
        };
        name = builtins.toString prNum;
      };
    };
  });
}
