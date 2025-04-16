{ pkgs ? import <nixpkgs> {}
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, utils ? import ./andrew-utils.nix {}
, fullMatrixOverride ? {}
}:
{ inlineJsonConfig
, inlineCommitList ? []
, prNum
}:
let
  jsonConfig = inlineJsonConfig // {
    gitCommits = map utils.srcFromCommit inlineCommitList;
  };
  qa-assets = builtins.fetchGit {
    url = "https://github.com/ElementsProject/qa-assets/";
    ref = "master";
  };
  fullMatrix = {
    projectName = "elements";
    inherit prNum;

    srcName = { src, ... }: src.commitId;
    mtxName = { src, withBench, withWallet, withDebug, check }:
      "elements-PR-${prNum}-${src.shortId}-${builtins.toString withBench}-${builtins.toString withWallet}-${builtins.toString withDebug}-${check}";

    withBench = [ true false ];
    withWallet = [ true false ];
    withDebug = [ true false ];

    src = jsonConfig.gitCommits;
  } // fullMatrixOverride;

  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";
    argsMatrix = fullMatrix;

    singleCheckDrv = {
      projectName,
      srcName,
      mtxName,
      withBench,
      withWallet,
      withDebug,
      check,
      src
    }: dummy:
    let
      drv = stdenv.mkDerivation {
        name = "${projectName}-${src.shortId}";
        src = src.src;

        nativeBuildInputs = [
          pkgs.pkgconfig
          pkgs.autoreconfHook
          pkgs.sqlite
        ] ++ lib.optionals (check != "") [
          pkgs.python3
        ] ++ lib.optionals (check == "fuzz") [
          pkgs.clang_15
        ];

        buildInputs = [
          pkgs.db48
          pkgs.boost
          pkgs.zlib
          pkgs.zeromq
          pkgs.miniupnpc
          pkgs.protobuf
          pkgs.libevent
        ] ++ lib.optionals stdenv.isLinux [
          pkgs.utillinux
        ];

        configureFlags = [
          "--with-boost-libdir=${pkgs.boost.out}/lib"
          "--with-sqlite-libdir=${pkgs.sqlite.out}/lib"
          "--with-sqlite=yes"
        ] ++ lib.optionals (!withBench) [
          "--disable-bench"
        ] ++ lib.optionals (!withWallet) [
          "--disable-wallet"
        ] ++ lib.optionals (check == "") [
          "--disable-tests"
          "--disable-gui-tests"
        ] ++ lib.optionals withDebug [
          "--enable-debug"
        ] ++ lib.optionals (check == "fuzz") [
          "--enable-fuzz"
          "--with-sanitizers=address,fuzzer,undefined"
          "--disable-asm"
          "CC=clang"
          "CXX=clang++"
        ];

        doCheck = check == "check";

        checkFlags = [ "LC_ALL=C.UTF-8" ];

        makeFlags = [ "VERBOSE=true" ];

        enableParallelBuilding = true;

        postInstall = if check == "fuzz"
        then ''
          cp -r ${qa-assets}/fuzz_seed_corpus .
          chmod +w -R fuzz_seed_corpus/
          patchShebangs test/fuzz
          ./test/fuzz/test_runner.py -j=$NIX_BUILD_CORES -l DEBUG fuzz_seed_corpus/
        ''
        else if check == "check" then ''
          patchShebangs test/functional
          ./test/functional/test_runner.py -j1
        ''
        else "";

        DIR_UNIT_TEST_DATA = "${qa-assets}/unit_test_data/";
        preCheck = ''
          echo "Using DIR_UNIT_TEST_DATA=$DIR_UNIT_TEST_DATA"
        '';

        meta = {
          homepage = http://www.github.com/ElementsProject/elements;
          license = lib.licenses.mit;
        };
      };
      taggedDrv = drv.overrideAttrs (self: {
        # Add a bunch of stuff just to make the derivation easier to grok
        checkPrProjectName = "elements";
        checkPrPrNum = prNum;
        checkPrWithBench = withBench;
        checkPrWithWallet = withWallet;
        checkPrWithDebug = withDebug;
        checkPrCheck = check;
        checkPrSrc = builtins.toJSON src;
      });
    in taggedDrv;
    
  };
in
  utils.checkPr checkData
