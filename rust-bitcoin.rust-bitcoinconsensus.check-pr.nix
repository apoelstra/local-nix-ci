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
      runClippy
      runFmt
      runDocs;

    features = [[] ["std"]]; # the external-secp feature will not work without a lot more work

    clippyExtraArgs = "-A clippy::doc_lazy_continuation"; # https://github.com/rust-bitcoin/rust-secp256k1/pull/705

    bitcoinRevFile = { src, ... }: builtins.elemAt (builtins.split "\n"
      (builtins.readFile "${src.src}/depend/bitcoin-HEAD-revision.txt"))
      2;
    bitcoinSrc = { bitcoinRevFile, ... }: builtins.fetchGit {
      allRefs = true;
      url = "https://github.com/bitcoin/bitcoin.git";
      rev = bitcoinRevFile;
    };

    extraTestPostRun = { bitcoinSrc, ... }:
      ''
        # crate2nix will symlinkify files in the workspace so we need to un-symlink
        # them in order for patchShebangs to work instead of silently failing.
        cp -L contrib/vendor-bitcoin-core.sh contrib/vendor-bitcoin-core-1.sh
        mv contrib/vendor-bitcoin-core-1.sh contrib/vendor-bitcoin-core.sh
        cp -L Cargo.toml Cargo.toml1
        mv Cargo.toml1 Cargo.toml
        cp -Lr src src2
        rm -r src
        mv src2 src

        # Check whether C code is consistent with upstream
        # Check whether C code is consistent with upstream
        patchShebangs ./contrib/vendor-bitcoin-core.sh
        mkdir depend2/
        cp depend/*.patch depend/check_uint128_t.c depend2/
            CORE_VENDOR_REPO=${bitcoinSrc} echo $CORE_VENDOR_REPO
        CORE_VENDOR_CP_NOT_CLONE=yes \
            CORE_VENDOR_GIT_ROOT=".." \
            CORE_VENDOR_REPO=${bitcoinSrc} \
            CORE_VENDOR_DEPEND_DIR=./depend2/ \
            ./contrib/vendor-bitcoin-core.sh -f  # use -f to avoid calling git in a non-git repo

        cp depend/bitcoin-HEAD-revision.txt depend2/
        # This specific file has some git-generated stuff that will not be
        # consistent between a git clone from rust-bitcoinconsensus and a
        # git clone from Bitcoin Core. So we whitelist this file when diffing.
        cp depend/bitcoin/src/clientversion.cpp depend2/bitcoin/src/clientversion.cpp
        # Bitcoin Core has several committed files which are gitignored, which
        # Nix drops due to a bug(?) in fetchGit. These files are not used by
        # the Rust bindings, so just manually patch them rather than investigating
        # WTF is going on. Note that these files *are* present in the Rust repo.
        # They are just not copied into the Nix store.
        # Also annoyingly this list changes from bitcoin rev to bitcoin rev.
        rm depend2/bitcoin/contrib/init/org.bitcoin.bitcoind.plist || true
        rm depend2/bitcoin/src/qt/Makefile || true
        rm depend2/bitcoin/src/qt/test/Makefile || true
        rm depend2/bitcoin/src/test/Makefile || true
        #rm -r depend2/bitcoin/contrib/guix/patches || true # out in 22, in in 23
        rm -r depend2/bitcoin/src/univalue/gen || true
        rm -r depend2/bitcoin/src/univalue/build-aux/m4/ax_cxx_compile_stdcxx.m4 || true
        rm -r depend2/bitcoin/src/minisketch/.cirrus.yml || true
        rm -r depend2/bitcoin/src/minisketch/src/test.cpp || true
        rm -r depend2/bitcoin/src/minisketch/tests || true
        rm -r depend2/bitcoin/.gitignore depend2/bitcoin/*/.gitignore depend2/bitcoin/*/*/.gitignore depend2/bitcoin/*/*/*/.gitignore || true
        # Ok. Actually do the diff.
        diff -r depend/ depend2/
      '';
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
