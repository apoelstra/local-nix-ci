{ pkgs ? import <nixpkgs> { }
, jsonConfigFile
, prNum
}:
let
  utils = import ./andrew-utils.nix { };
  jsonConfig = utils.parseRustConfig { inherit jsonConfigFile prNum; };

  fullMatrix = {
    inherit prNum;
    inherit (utils.standardRustMatrixFns jsonConfig)
      projectName src rustc lockFile srcName mtxName
      isMainLockFile isMainWorkspace mainCargoToml workspace cargoToml
      features # Must be overridden if there are any exceptional feature combinations
      runClippy
      runFmt
      runDocs;

    clippyExtraArgs = "-A clippy::doc_lazy_continuation"; # https://github.com/rust-bitcoin/rust-secp256k1/pull/705

    secp256k1RevFile = { src, ... }: builtins.elemAt (builtins.split "\n"
      (builtins.readFile "${src.src}/secp256k1-sys/depend/secp256k1-HEAD-revision.txt"))
      2;
    secp256k1Src = { secp256k1RevFile, ... }: builtins.fetchGit {
      allRefs = true;
      url = "https://github.com/bitcoin-core/secp256k1/";
      rev = secp256k1RevFile;
    };

    extraTestPostRun = { workspace, secp256k1Src, ... }:
      if workspace == "secp256k1-sys"
      then ''
        # crate2nix will symlinkify files in the workspace so we need to un-symlink
        # them in order for patchShebangs to work instead of silently failing.
        cp -L vendor-libsecp.sh vendor-libsecp-1.sh
        mv vendor-libsecp-1.sh vendor-libsecp.sh
        cp -L Cargo.toml Cargo.toml1
        mv Cargo.toml1 Cargo.toml
        cp -Lr src src2
        rm -r src
        mv src2 src

        # Check whether C code is consistent with upstream
        patchShebangs ./vendor-libsecp.sh
        sed -i "s#^SECP_SYS=.*#SECP_SYS=$PWD#" ./vendor-libsecp.sh
        sed -i "s#set -e#set -ex#" ./vendor-libsecp.sh
        mkdir depend2/
        cp depend/*.patch depend/check_uint128_t.c depend2/
        SECP_VENDOR_CP_NOT_CLONE=yes \
            SECP_VENDOR_GIT_ROOT=".." \
            SECP_VENDOR_SECP_REPO=${secp256k1Src} \
            SECP_VENDOR_DEPEND_DIR=./depend2/ \
            ./vendor-libsecp.sh -f  # use -f to avoid calling git in a non-git repo

        cp depend/secp256k1-HEAD-revision.txt depend2/
        rm depend2/secp256k1/.gitignore # dropped by crate2nix I think
        rm depend/secp256k1/*/*.orig || true # These files are weird seem to depend on `diff` weirdness
        rm depend2/secp256k1/*/*.orig || true
        diff -r depend/ depend2
      ''
      else "";
  };

  checkData = rec {
    name = "${jsonConfig.projectName}-pr-${builtins.toString prNum}";
    argsMatrix = fullMatrix;
    singleCheckDrv = utils.crate2nixSingleCheckDrv;
    memoGeneratedCargoNix = utils.crate2nixMemoGeneratedCargoNix;
    memoCalledCargoNix = utils.crate2nixMemoCalledCargoNix;
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr checkData;
}
