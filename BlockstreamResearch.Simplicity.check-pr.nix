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
  gitCommits = utils.githubPrSrcs {
    # This must be a .git directory, not a URL or anything, since githubPrCommits
    # well set the GIT_DIR env variable to it before calling git commands. The
    # intention is for this to be run locally.
    gitDir = /. + jsonConfig.gitDir;
    gitUrl = jsonConfig.gitUrl;
    inherit prNum;
  };
  benchmarkSrc = builtins.fetchurl {
    url = "https://gist.githubusercontent.com/sanket1729/0bf92ab9b2d17895d4afdfe3a85bdf70/raw/a0c8cf0f08e07945d8fcc04640bf567a9ba9f368/jet_benches.json";
    sha256 = "1f2gdgvr5lfj3anzrs13hhcvdyfcz6dy2vy5p136hn5z3kzidnhk";
  };
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      {
        srcName = self: self.src.commitId;
        mtxName = self: "${self.src.shortId}-${self.attr}";

        attr = [ "coq" "haskell" "compcert" "vst" "pdf" ];
#        attr = [ "coq" ];
        doCheck = null;
        wideMultiply = null;
        withCoverage = null;
        production = null;
        env = null;
        src = gitCommits;
      }

      {
        srcName = self: self.src.commitId;
        mtxName = self: "${self.src.shortId}-${self.attr}-${builtins.toString self.wideMultiply}-${self.env}-cov-${builtins.toString self.withCoverage}";

        attr = "c";
        doCheck = [ false true ];
        wideMultiply = [ null "int64" "int128" "int128_struct" ];
        withCoverage = [ false true ];
        production = [ false true ];
        env = [ "stdenv" "clangStdenv" ];
        src = gitCommits;
      }
    ];

    singleCheckDrv = { src, attr, doCheck, wideMultiply, withCoverage, production, env, srcName, mtxName }: dummy:
      let
        sourceDir = src.src;
        drv = builtins.getAttr attr (import "${sourceDir}/default.nix" {
          inherit doCheck wideMultiply withCoverage production;
          withProfiler = withCoverage;
          withValgrind = !withCoverage; # The coverage tool does some bad memory things so it must be exclusive with valgrind
          withTiming = false; # !withValgrind; # just leave at false since this is so flakey for me
        });
        diffDrv = if attr == "haskell"
        then
          drv.overrideAttrs
            (self: super: {
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

                echo "Checking benchmarks. WARNING whitelisting GeNegate which has made-up value"
                if grep -q rawBenchmark Haskell-Generate/GenPrimitive.hs; then
                    # head -c -1 is trick to eat the trailing newline from the heredoc
                    head -c -1 <<EOT | diff ${benchmarkSrc} - || true
                {
                $(
                    grep 'rawBenchmark.*=' Haskell-Generate/GenPrimitive.hs \
                        | grep -v '^rawBenchmark str = error' \
                        | sed 's/rawBenchmark \(".*"\) = \(.*\)/  \1: \2,/' \
                        | sed 's/\(  "Version":.*\),/\1/' \
                        | grep -v 'GeNegate'
                )
                }
                EOT
                fi
              '';
            })
        else drv;
        taggedDrv = diffDrv.overrideAttrs (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = "Simplicity";
          checkPrAttr = attr;
          checkPrPrNum = prNum;
          checkPrWideMultiply = wideMultiply;
          checkPrProduction = production;
          checkPrEnv = builtins.toJSON env;
          checkPrSrc = builtins.toJSON src;
        });
      in taggedDrv;
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr (checkData // {
    argsMatrices = map
      (argsMtx: argsMtx // {
        src = rec {
          src = builtins.fetchGit {
            allRefs = true;
            url = jsonConfig.gitDir;
            rev = singleRev;
          };
          name = builtins.toString prNum;
          shortId = name;
          commitId = shortId;
        };
      })
      checkData.argsMatrices;
  });
}
