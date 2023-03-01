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
  utils = import ../../nix-setup/scrap/andrew-utils.nix {};
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
      attr = [ "c" "coq" "haskell" "compcert" "vst" ];
      src = map (commit: {
        src = builtins.fetchGit {
          url = gitUrl;
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
          url = gitUrl;
          ref = prNum;
        };
        name = builtins.toString prNum;
      };
    };
  });
}
