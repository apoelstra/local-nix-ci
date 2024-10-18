let
  pkgs = import <nixpkgs> {};
  utils = import ./andrew-utils.nix {};
in import ./rust.check-pr.nix {
  inherit pkgs utils;
  fullMatrixOverride = {
    secp256k1RevFile = { src, ... }: builtins.elemAt (builtins.split "\n"
      (builtins.readFile "${src.src}/secp256k1-zkp-sys/depend/secp256k1-HEAD-revision.txt"))
      2;
    secp256k1Src = { secp256k1RevFile, ... }: builtins.fetchGit {
      allRefs = true;
      url = "https://github.com/ElementsProject/secp256k1-zkp/";
      rev = secp256k1RevFile;
    };

    extraTestPostRun = { isMainLockFile, workspace, rustc, secp256k1Src, ... }:
      pkgs.lib.optionalString (isMainLockFile && workspace == "secp256k1-zkp-sys" && utils.rustcIsNightly rustc) ''
        # Check whether C code is consistent with upstream
        pushd secp256k1-zkp-sys
        patchShebangs ./vendor-libsecp.sh
        mkdir depend2/
        cp depend/*.patch depend/check_uint128_t.c depend2/
        SECP_VENDOR_CP_NOT_CLONE=yes \
            SECP_VENDOR_GIT_ROOT=".." \
            SECP_VENDOR_SECP_REPO=${secp256k1Src} \
            SECP_VENDOR_DEPEND_DIR=./depend2/ \
            ./vendor-libsecp.sh -f  # use -f to avoid calling git in a non-git repo

        cp depend/secp256k1-HEAD-revision.txt depend2/
        diff -r depend/ depend2
        popd
      '';
  };
}
