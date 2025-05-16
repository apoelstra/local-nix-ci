{ pkgs ? import <nixpkgs> { }
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, jsonConfigFile ? null
, inlineJsonConfig ? null
, inlineCommitList ? []
, prNum
}:
let
  utils = import ./andrew-utils.nix { };
  jsonConfig = if builtins.isNull inlineJsonConfig
    then utils.parseRustConfig { inherit jsonConfigFile prNum; }
    else inlineJsonConfig // {
        gitCommits = map utils.srcFromCommit inlineCommitList;
    };
  benchmarkSrc = builtins.fetchurl {
    url = "https://gist.githubusercontent.com/sanket1729/0bf92ab9b2d17895d4afdfe3a85bdf70/raw/a0c8cf0f08e07945d8fcc04640bf567a9ba9f368/jet_benches.json";
    sha256 = "1f2gdgvr5lfj3anzrs13hhcvdyfcz6dy2vy5p136hn5z3kzidnhk";
  };
  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";

    argsMatrix = {
      srcName = { src, ... }: src.commitId;
      mtxName = { src, attr, wideMultiply, env, withCoverage, ... }: if attr == "c"
        then "${src.shortId}-${attr}-${builtins.toString wideMultiply}-${env}-cov-${builtins.toString withCoverage}"
        else "${src.shortId}-${attr}";

      attr = [ "c" "coq" "haskell" "compcert" "vst" "pdf" ];
      use686 = { attr, ... }: if attr == "c" then [ false true ] else false;
      doCheck = { attr, ... }: if attr == "c" then [ false true ] else true;
      wideMultiply = { attr, use686, ... }: if attr == "c"
        then if use686
          then [ null ] # "int64" ] # int64 disabled since #256 for all configs
          else [ null "int128" "int128_struct" ]
        else null;
      withCoverage = { attr, ... }: if attr == "c" then [ false true ] else null;
      withValgrind = { attr, withCoverage, ... }: if attr == "haskell" then [ false true ] else withCoverage != true;
      production = { attr, ... }: if attr == "c" then [ false true ] else null;
      env = { attr, ... }: if attr == "c" then [ "stdenv" "clangStdenv" ] else null;
      src = jsonConfig.gitCommits;
    };

    singleCheckDrv = {
      src,
      attr,
      use686,
      doCheck,
      wideMultiply,
      withCoverage,
      withValgrind,
      production,
      env,
      srcName,
      mtxName
    }: dummy1: dummy2:
      let
        sourceDir = src.src;
        nixpkgs = if use686 then pkgs.pkgsi686Linux else pkgs;
        drv = builtins.getAttr attr (import "${sourceDir}/default.nix" {
          inherit nixpkgs doCheck wideMultiply withCoverage production;
          withProfiler = withCoverage;
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
                    diff "$inc" "${sourceDir}/C/elements/$inc"
                done
                rm ./*.inc

                #./dist/build/GenRustJets/GenRustJets # Output not committed anywhere
                ./dist/build/GenTests/GenTests
                for inc in checkSigHashAllTx1.[ch]; do
                    diff "$inc" "${sourceDir}/C/elements/$inc"
                done
                rm checkSigHashAllTx1.[ch]

                for inc in *.[ch]; do
                    diff "$inc" "${sourceDir}/C/$inc"
                done
                rm ./*.[ch]

                ./dist/build/GenDecodeJet/GenDecodeJet
                for inc in *.inc; do
                    case $inc in
                    decodeCoreJets.inc)
                        diff "$inc" "${sourceDir}/C/$inc"
                        ;;
                    decodeElementsJets.inc)
                        diff "$inc" "${sourceDir}/C/elements/$inc"
                        ;;
                    *)
                        echo "Unexpected output $inc from GenDecodeJet"
                        exit 1
                        ;;
                    esac
                done
                rm ./*.inc

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
  checkHead = utils.checkPr checkData;
}
