{
  pkgs ? import <nixpkgs> {}
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
  boost = pkgs.boost175;
  qa-assets = builtins.fetchGit {
    url = "https://github.com/ElementsProject/qa-assets/";
#    ref = "master";
    allRefs = true;
    rev = "26fd9bbf76e30d4566e135141f0385a5470fe88b";
  };
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [{
      projectName = "elements";
      srcName = self: self.src.commitId;
      mtxName = self: "${self.projectName}-PR-${prNum}-${self.src.shortId}-${builtins.toString self.withBench}-${builtins.toString self.withWallet}-${builtins.toString self.withDebug}-${self.check}";

      withBench = [ true false ];
      withWallet = [ true false ];
#      withWallet = [ true ];
      withDebug = [ true false ];
      check = [ "" "check" "fuzz" ];

      src = gitCommits;
    }];

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
          boost  # NOT pkgs.boost
          pkgs.zlib
          pkgs.zeromq
          pkgs.miniupnpc
          pkgs.protobuf
          pkgs.libevent
        ] ++ lib.optionals stdenv.isLinux [
          pkgs.utillinux
        ];

        configureFlags = [
          "--with-boost-libdir=${boost.out}/lib"
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
