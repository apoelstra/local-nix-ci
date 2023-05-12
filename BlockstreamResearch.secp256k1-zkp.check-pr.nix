{
  pkgs ? import <nixpkgs> {}
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, jsonConfigFile
, prNum
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
  extraModulesName = mods: builtins.concatStringsSep "_" (map (builtins.substring 0 4) mods);
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [{
      projectName = "libsecp256k1-zkp";
      srcName = self: self.src.commitId;
      mtxName = self: "${self.projectName}-PR-${prNum}-${self.src.shortId}-${self.withAsm}-${extraModulesName self.extraModules}";

      extraModules = [
        []
        ["recovery"]
        ["ecdh"]
        ["musig" "extrakeys" "schnorrsig"]
        ["generator"]
        ["generator" "rangeproof" "surjectionproof" "whitelist"]
        ["recovery" "ecdh" "musig" "extrakeys" "schnorrsig" "generator" "rangeproof" "surjectionproof" "whitelist"]
      ];
      ecmultGenPrecision = [ 2 4 8 ];
      ecmultWindow = [ 2 15 20 ]; # clang can't handle 24 :(:
      withAsm = [ "no" "x86_64" ];
      withMsan = [ true false ];
      widemul = [ "int64" "int128" "int128_struct" ];
      doValgrindCheck = true;
      src = gitCommits;
    }];

    singleCheckDrv = {
      projectName,
      srcName,
      mtxName,
      extraModules,
      ecmultGenPrecision,
      ecmultWindow,
      withAsm,
      withMsan,
      widemul,
      doValgrindCheck,
      src
    }: dummy:
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
      drv = stdenv.mkDerivation {
        name = "${projectName}-${src.shortId}";
        src = src.src;

        nativeBuildInputs = [ pkgs.pkgconfig pkgs.autoreconfHook pkgs.valgrind ]
          ++ lib.optionals withMsan [
            pkgs.llvmPackages_15.llvm # to get llvm-symbolizer when clang blows up
            pkgs.clang_15
          ];
        buildInputs = [];

        configureFlags = [
          "--with-ecmult-gen-precision=${builtins.toString ecmultGenPrecision}"
          "--with-ecmult-window=${builtins.toString ecmultWindow}"
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
          if [ ${builtins.toString ecmultWindow} -gt 15 ]; then
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
        checkPrProjectName = "libsecp256k1-zkp";
        checkPrPrNum = prNum;
        checkPrExtraModules = builtins.toJSON extraModules;
        checkPrEcmultGenPrecision = ecmultGenPrecision;
        checkPrEcmultWindow = ecmultWindow;
        checkPrWithAsm = withAsm;
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
            rev = prNum;
          };
          name = builtins.toString prNum;
          shortId = name;
          commitId = shortId;
        };
      })
      checkData.argsMatrices;
  });
}
