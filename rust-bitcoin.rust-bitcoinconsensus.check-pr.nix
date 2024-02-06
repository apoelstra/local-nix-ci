{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
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
  nightlyRustc = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
  allRustcs = [
    nightlyRustc
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.48.0".default
  ];
  gitCommits = utils.githubPrSrcs {
    # This must be a .git directory, not a URL or anything, since githubPrCommits
    # well set the GIT_DIR env variable to it before calling git commands. The
    # intention is for this to be run locally.
    gitDir = /. + jsonConfig.gitDir;
    gitUrl = jsonConfig.gitUrl;
    inherit prNum;
  };
  lockFileName = attrs: builtins.unsafeDiscardStringContext (builtins.baseNameOf (attrs.lockFileFn attrs.src));
  lockFileFn = [
    (src: "${src.src}/Cargo-minimal.lock")
    (src: "${src.src}/Cargo-recent.lock")
  ];
  srcName = self: self.src.commitId;
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${lockFileName self}-${builtins.concatStringsSep "," self.features}";
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      {
        projectName = jsonConfig.repoName;
        inherit srcName mtxName prNum lockFileFn;

        isTip = false;

        features = [
          [ ]
          [ "std" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }

      # Only tip
      {
        projectName = jsonConfig.repoName;
        inherit srcName prNum lockFileFn;

        isTip = true;

        features = [ [] [ "std" ] ];
        rustc = nightlyRustc;
        src = builtins.head gitCommits;

        mtxName = self: (mtxName self) + "-tip";
      }
    ];

    singleCheckMemo = attrs:
      let tweakAttrs = attrs // { lockFile = attrs.lockFileFn attrs.src; };
      in utils.crate2nixSingleCheckMemo tweakAttrs;

    singleCheckDrv =
      { projectName
      , prNum
      , isTip
      , features
      , rustc
      , lockFileFn
      , src
      , srcName
      , mtxName
      ,
      }:
      nixes:
        let
          vendorRevFile = builtins.readFile "${src.src}/depend/bitcoin-HEAD-revision.txt";
          vendorSrc = builtins.fetchGit {
            allRefs = true;
            url = "https://github.com/bitcoin/bitcoin.git";
            rev = builtins.elemAt (builtins.split "\n" vendorRevFile) 2;
          };
          drv = nixes.called.rootCrate.build.override {
            inherit features;
            runTests = true;
            testPreRun = ''
              ${rustc}/bin/rustc -V
              ${rustc}/bin/cargo -V
              echo "Tip: ${builtins.toString isTip}"
              echo "PR: ${prNum}"
              echo "Commit: ${src.commitId}"
              echo "Features: ${builtins.toJSON features}"
            '';
          };
          finalDrv = stdenv.mkDerivation {
            name = projectName;
            src = src.src;
            buildInputs = [
              rustc    # not from pkgs; this is an arg to singleCheckDrv
            ];
            phases = [ "unpackPhase" "buildPhase" ];

            buildPhase = ''
              set -x
              cargo -V
              echo "Source: ${builtins.toJSON src}"

              # Run clippy/fmt checks
              export CARGO_TARGET_DIR=$PWD/target
              export CARGO_HOME=${nixes.generated}/cargo
              pushd ${nixes.generated}/crate
              cargo clippy --locked -- -D warnings
              cargo fmt --all -- --check
              popd

              # Check whether C code is consistent with upstream
              patchShebangs ./contrib/vendor-bitcoin-core.sh
              mkdir depend2/
              cp depend/*.patch depend/check_uint128_t.c depend2/
                  CORE_VENDOR_REPO=${vendorSrc} echo $CORE_VENDOR_REPO
              CORE_VENDOR_CP_NOT_CLONE=yes \
                  CORE_VENDOR_GIT_ROOT=".." \
                  CORE_VENDOR_REPO=${vendorSrc} \
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
              rm -r depend2/bitcoin/src/minisketch/.gitignore || true
              rm -r depend2/bitcoin/src/minisketch/src/test.cpp || true
              rm -r depend2/bitcoin/src/minisketch/tests || true
              # Ok. Actually do the diff.
              diff -r depend/ depend2/

              touch $out
            '';
          };
        in
        if isTip
        then finalDrv
        else drv.overrideDerivation (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = projectName;
          checkPrPrNum = prNum;
          checkPrRustc = rustc;
          checkPrFeatures = builtins.toJSON features;
          checkPrSrc = builtins.toJSON src;
        });
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

