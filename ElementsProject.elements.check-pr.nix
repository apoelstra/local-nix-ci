{ pkgs ? import <nixpkgs> {}
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
  qa-assets = builtins.fetchGit {
    url = "https://github.com/ElementsProject/qa-assets/";
    ref = "master";
  };
  fullMatrix = {
    projectName = "elements";
    inherit prNum;

    srcName = { src, ... }: src.commitId;
    mtxName = { src, withBench, withWallet, withDebug, check, ... }:
      "elements-PR-${prNum}-${src.shortId}-${builtins.toString withBench}-${builtins.toString withWallet}-${builtins.toString withDebug}-${check}";

    check = [ "" "check" "fuzz" ];
    withBench = [ true false ];
    withWallet = [ true false ];
    withSqlite = { withWallet, ... }: if withWallet
      then [ true false ]
      else [ false ]; # enabling sqlite with --disable-wallet causes functional tests to fail
    withDebug = [ true false ];

    src = jsonConfig.gitCommits;
  };

  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";
    argsMatrix = fullMatrix;
    forceSequential = true; # see docs in andrew-utils.nix for what this does

    singleCheckDrv = {
      projectName,
      prNum,
      srcName,
      mtxName,
      withBench,
      withWallet,
      withSqlite,
      withDebug,
      check,
      src
    }:
    dummy1:  # generated cargo.nix
    dummy2:  # called cargo.nix
    let
      drv = stdenv.mkDerivation {
        name = "${projectName}-${src.shortId}";
        src = src.src;

        nativeBuildInputs = [
          pkgs.pkg-config
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
        ] ++ lib.optionals (withSqlite) [
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
        patches = [
          ./patches/elements-001.patch # https://github.com/bitcoin/bitcoin/pull/29823
          ./patches/elements-002.patch # increase timeout for unit test
          ./patches/elements-003.patch # https://github.com/ElementsProject/elements/pull/1450
        ];

        postInstall = if check == "fuzz"
        then ''
          cp -r ${qa-assets}/fuzz_seed_corpus .
          chmod +w -R fuzz_seed_corpus/
          patchShebangs test/fuzz
          cat test/sanitizer_suppressions/ubsan
          ./test/fuzz/test_runner.py -j=$NIX_BUILD_CORES -l DEBUG fuzz_seed_corpus/
        ''
        else if check == "check" then
         # Disable functional tests when debug is on because it makes everything too slow.
          if withDebug
          then ''
            echo "Skipping functional tests because debug build is on."
          ''
          else ''
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
