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
  gitCommits = utils.githubPrSrc {
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
    argsMatrices = [{
      attr = [ "c" "coq" "haskell" "compcert" "vst" ];
      src = gitCommits;
    }];
  
    checkSingleCommit = { src, attr }:
      let
        sourceDir = src.src;
        drv = builtins.getAttr attr (import "${sourceDir}/default.nix" {});
        tweakedDrv = if attr == "haskell"
          then drv.overrideAttrs (self: super: {
            postBuild = ''
              set -ex
              echo "Running all code-generation steps and checking output against checked-in files."
              ./dist/build/GenPrecomputed/GenPrecomputed
              diff "precomputed.h" ${sourceDir}/C/precomputed.h
              rm "precomputed.h"
  
              ./dist/build/GenPrimitive/GenPrimitive
              for inc in *.inc; do
                  diff "$inc" "${sourceDir}/C/primitive/elements/$inc"
              done
              rm ./*.inc
  
              #./dist/build/GenRustJets/GenRustJets # Output not committed anywhere
              ./dist/build/GenTests/GenTests
              for inc in checkSigHashAllTx1.[ch]; do
                  diff "$inc" "${sourceDir}/C/primitive/elements/$inc"
  	    done
  	    rm checkSigHashAllTx1.[ch]
  
              for inc in *.[ch]; do
                  diff "$inc" "${sourceDir}/C/$inc"
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
