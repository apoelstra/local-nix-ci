{
  pkgs ? import <nixpkgs> {}
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, utils ? import ./andrew-utils.nix {}
, inlineJsonConfig
, inlineCommitList ? []
, prNum
}:
let
  jsonConfig = inlineJsonConfig // {
    gitCommits = map utils.srcFromCommit inlineCommitList;
  };
  extraModulesName = mods: builtins.concatStringsSep "_" (map (builtins.substring 0 4) mods);
  fullMatrix = {
    projectName = "secp256k1";
    inherit prNum;

    srcName = { src, ... }: src.commitId;
    mtxName = { src, withAsm, extraModules, ...}:
      "libsecp-PR-${prNum}-${src.shortId}-${withAsm}-${extraModulesName extraModules}";

    extraModules = [
      []
      ["ecdh"]
      ["ellswift"]
      ["extrakeys"]
      ["musig"]
      ["recovery"]
      ["schnorrsig"]
      [ "ecdh" "ellswift" "extrakeys" "musig" "schnorrsig" ]
      [ "ecdh" "ellswift" "extrakeys" "musig" "recovery" "schnorrsig" ]
    ];

    ecmultGenKb = [ 2 22 86 ];
    # z-prefix this to try to spread out the 24-bit instances in the matrix
    zecmultWindow = [
      2 10 15
      # Frustratingly my system OOMs when dealing with more than one commit
      # at 24-bit ecmult windows. Even with 1TB of RAM. So throttle it in
      # the multi-commit case but leave it be for the "just testing HEAD"
      # case.
      (if builtins.length jsonConfig.gitCommits > 1
      then 21
      else 24)
    ];
    withAsm = [ "no" "x86_64" ];
    withMsan = [ true false ];
    widemul = [ "int64" "int128" "int128_struct" ];
    doValgrindCheck = true;
    src = jsonConfig.gitCommits;
  };

  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";
    argsMatrix = fullMatrix;
    # See docs in andrew-utils.nix for these parameters.
    forceSequential = true;
    # We can set the sequentialWidth pretty high, because it's interpreted as a batch size, and
    # Nix will do a complete batch before starting the next. Not very many high-memory derivations
    # will show up in a given batch, and even if they did, it's okay since as they complete the
    # system load will reduce and we'll recover.
    #
    # Without forceSequential, every time a derivation finishes Nix immediately spawns a new one
    # so that it's always at/near max-jobs. This means that no matter the initial distribution
    # of jobs, it'll wind up doing max-jobs many of the longest-running ones at once... which are
    # exactly the high-memory ones. So all we need to do to avoid memory exhaustion is to disrupt
    # that process.
    sequentialWidth = 128;

    singleCheckDrv = {
        projectName
      , prNum
      , srcName
      , mtxName
      , extraModules
      , ecmultGenKb
      , zecmultWindow
      , withAsm
      , withMsan
      , widemul
      , doValgrindCheck
      , src
    }:
    dummy1:  # generated cargo.nix
    dummy2:  # called cargo.nix
    let
      valgrindCheckCmd = if doValgrindCheck
        then ''
          valgrind ./exhaustive_tests 1
          valgrind ./tests 1
        ''
        else "";
      ctimeCheckCmd = if withMsan
        then ''
          if [ -f ./ctime_tests ]; then
            ./ctime_tests
          fi
        ''
        else ''
          if [ -f ./ctime_tests ]; then
            libtool --mode=execute valgrind ./ctime_tests
          fi
        '';
      # clang can't seem to handle ecmult windows > 20 :(
      adjEcmultWindow = if withMsan && zecmultWindow > 20
        then zecmultWindow - 4
        else zecmultWindow;
      drv = stdenv.mkDerivation {
        name = "${projectName}-${src.shortId}";
        src = src.src;
  
        nativeBuildInputs = [ pkgs.pkg-config pkgs.autoreconfHook pkgs.valgrind ]
          ++ lib.optionals withMsan [
            pkgs.llvmPackages_20.llvm # to get llvm-symbolizer when clang blows up
            pkgs.clang_20
          ];
        buildInputs = [];
  
        configureFlags = [
          "--with-ecmult-gen-kb=${builtins.toString ecmultGenKb}"
          "--with-ecmult-window=${builtins.toString adjEcmultWindow}"
          "--with-test-override-wide-multiply=${widemul}"
        ] ++ (if withMsan
          then [ "CC=clang" "--without-asm" "CFLAGS=-fsanitize=memory" ]
          else [ "--with-asm=${withAsm}" ]
        ) ++ (if builtins.length extraModules > 0
          then [ "--enable-experimental" ] ++ (map (x: "--enable-module-${x}") extraModules)
          else []
        );
  
        postUnpack = ''
          # See comment in this file; for ecmult windows > 15 we need to delete
          # it so that it can be regenerated.
          if [ ${builtins.toString adjEcmultWindow} -gt 15 ]; then
            rm ./source/src/precomputed_ecmult.c
          fi
        '';
        postCheck = ctimeCheckCmd + valgrindCheckCmd;
        makeFlags = [ "VERBOSE=true" ];
  
        # TODO turn this off when the new RAM arrives
        enableParallelBuilding = false;
  
        meta = {
          homepage = http://www.github.com/bitcoin-core/secp256k1;
          license = lib.licenses.mit;
        };
      };
      taggedDrv = drv.overrideAttrs (self: {
        # Add a bunch of stuff just to make the derivation easier to grok
        checkPrProjectName = "libsecp256k1";
        checkPrPrNum = prNum;
        checkPrExtraModules = builtins.toJSON extraModules;
        checkPrEcmultGenKb = ecmultGenKb;
        checkPrEcmultWindow = adjEcmultWindow;
        checkPrWithAsm = withAsm;
        checkPrSrc = builtins.toJSON src;
      });
    in taggedDrv;
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr checkData;
}
