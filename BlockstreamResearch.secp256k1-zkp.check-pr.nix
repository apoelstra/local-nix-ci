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
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [{
      projectName = "libsecp256k1-zkp";
      srcName = self: self.src.commitId;
      mtxName = self: "${self.projectName}-PR-${prNum}-${self.src.shortId}-${self.withAsm}-${self.withBigNum}-${builtins.toString self.extraModules}";

      extraModules = [
        []
        ["recovery"]
      ];
      ecmultGenPrecision = [ 2 4 8 ];
      ecmultWindow = [ 2 15 24 ];
      withAsm = [ "no" "x86_64" ];
      withBigNum = [ "no" "gmp" ];
      withMsan = [ true false ];
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
      withBigNum,
      withMsan,
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
          ++ lib.optionals withMsan [ pkgs.clang_15 ];
        buildInputs = [];
    
        configureFlags = [
          "--with-bignum=${withBigNum}"
          "--with-ecmult-gen-precision=${builtins.toString ecmultGenPrecision}"
          "--with-ecmult-window=${builtins.toString ecmultWindow}"
        ] ++ (if withMsan
          then [ "CC=clang" "--without-asm" "CFLAGS=-fsanitize=memory" ]
          else [ "--with-asm=${withAsm}" ]
        ) ++ (if builtins.length extraModules > 0
          then [ "--enable-experimental" ] ++ (map (x: "--enable-module-${x}") extraModules)
          else []
        );
    
        postCheck = ctimeCheckCmd + valgrindCheckCmd;
        makeFlags = [ "VERBOSE=true" ];
    
        enableParallelBuilding = true;
    
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
        checkPrWithBigNum = withBigNum;
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
