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
      attr = [ "c" "coq" "haskell" "compcert" "vst" ];
      src = map (commit: {
        src = builtins.fetchGit {
          url = jsonConfig.gitUrl;
          ref = "refs/pull/${builtins.toString prNum}/head";
          rev = commit;
        };
        name = builtins.substring 0 8 commit;
      }) gitCommits;
    };
  
    checkSingleCommit = { src, attr }:
      let
        source = src.src.outPath;
        drv = builtins.getAttr attr (import "${source}/default.nix" {});
        tweakedDrv = if attr == "haskell"
          then drv.overrideAttrs (self: super: {
            postBuild = ''
              set -ex
              echo "Running all code-generation steps and checking output against checked-in files."
              ./dist/build/GenPrecomputed/GenPrecomputed
              diff "precomputed.h" ${source}/C/precomputed.h
              rm "precomputed.h"
  
              ./dist/build/GenPrimitive/GenPrimitive
              for inc in *.inc; do
                  diff "$inc" "${source}/C/primitive/elements/$inc"
              done
              rm ./*.inc
  
              #./dist/build/GenRustJets/GenRustJets # Output not committed anywhere
              ./dist/build/GenTests/GenTests
              for inc in checkSigHashAllTx1.[ch]; do
                  diff "$inc" "${source}/C/primitive/elements/$inc"
  	    done
  	    rm checkSigHashAllTx1.[ch]
  
              for inc in *.[ch]; do
                  diff "$inc" "${source}/C/$inc"
              done
              rm ./*.[ch]
            ''; })
          else drv;
        namedDrv = tweakedDrv.overrideAttrs (self: super: {
          name = "simplicity-pr-" + (builtins.toString prNum) + "-" + attr + "-" + src.name;
        });
      in namedDrv;
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
