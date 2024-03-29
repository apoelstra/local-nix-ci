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
  tools-nix = pkgs.callPackage utils.tools-nix-path { };
  jsonConfig = lib.trivial.importJSON jsonConfigFile;
  allRustcs = map
  (tchain: tchain.override { targets = [ "wasm32-unknown-unknown" ]; })
  [
    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.48.0".default
  ];
  isNightly = rustc: rustc == builtins.head allRustcs;
  gitCommits = utils.githubPrSrcs {
    # This must be a .git directory, not a URL or anything, since githubPrCommits
    # well set the GIT_DIR env variable to it before calling git commands. The
    # intention is for this to be run locally.
    gitDir = /. + jsonConfig.gitDir;
    gitUrl = jsonConfig.gitUrl;
    inherit prNum;
  };
  lockFileName = attrs: builtins.unsafeDiscardStringContext (builtins.baseNameOf (attrs.lockFileFn attrs.src));
  srcName = self: self.src.commitId;
  mtxName = self: "${self.src.shortId}-${self.workspace}-${self.rustc.name}-${lockFileName self}-${builtins.concatStringsSep "," (map (name: builtins.substring 0 8 name) self.features)}";
  lockFileFn = [
    (src: "${src.src}/Cargo-minimal.lock")
    (src: "${src.src}/Cargo-recent.lock")
  ];
  isTip = src: src == builtins.head gitCommits;

  libsecpSrc = fetchGit {
    url = "https://github.com/bitcoin-core/secp256k1/";
    ref = "master";
  };

  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      # Main project
      rec {
        projectName = jsonConfig.repoName;
        inherit isTip srcName mtxName prNum lockFileFn;

        workspace = "secp256k1";
        features = [
          [ ]
          [ "std" ]
          [ "alloc" ]
          [ "hashes" ]
          [ "hashes-std" ]
          [ "rand" ]
          [ "rand-std" ]
          [ "recovery" ]
          [ "lowmemory" ]
          [ "serde" ]
          [ "global-context" ]
          [ "global-context-less-secure" ]
          [ "global-context" "global-context-less-secure" ]
          [ "std" "hashes" "hashes-std" "rand" "rand-std" "recovery" "lowmemory" "global-context" "global-context-less-secure" "serde" ]
          [ "hashes" "rand" "recovery" "lowmemory" "global-context" "global-context-less-secure" "serde" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }


      # secp256k1-sys
      rec {
        projectName = jsonConfig.repoName;
        inherit isTip srcName mtxName prNum lockFileFn;

        workspace = "secp256k1-sys";
        features = [
          [ ]
          [ "lowmemory" ]
          [ "recovery" ]
          [ "alloc" ]
          [ "std" ]
          [ "std" "lowmemory" "recovery" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }

      # single checks
      {
        projectName = "final-checks";
        inherit isTip srcName mtxName prNum lockFileFn;

        workspace = "secp256k1";
        features = [ [] ];
        rustc = builtins.head allRustcs;
        src = gitCommits;
      }
    ];

    singleCheckMemo = attrs:
      let tweakAttrs = attrs // { lockFile = attrs.lockFileFn attrs.src; };
      in utils.crate2nixSingleCheckMemo tweakAttrs;

    singleCheckDrv =
      { projectName
      , prNum
      , isTip
      , workspace
      , features
      , rustc
      , lockFileFn
      , src
      , srcName
      , mtxName
      ,
      }:
      nixes:
        with pkgs;
        let
          pkgs = import <nixpkgs> {
            overlays = [ (self: super: { inherit rustc; }) ];
          };
          libsecpRevFile = builtins.readFile "${src.src}/secp256k1-sys/depend/secp256k1-HEAD-revision.txt";
          libsecpSrc = builtins.fetchGit {
            allRefs = true;
            url = "https://github.com/bitcoin-core/secp256k1/";
            rev = builtins.elemAt (builtins.split "\n" libsecpRevFile) 2;
          };
          drv = nixes.called.workspaceMembers.${workspace}.build.override {
            inherit features;
            runTests = true;
            testPreRun = ''
              ${rustc}/bin/rustc -V
              ${rustc}/bin/cargo -V
              echo "Source: ${builtins.toJSON src}"
              echo "Features: ${builtins.toJSON features}"
            '';
          };
          finalDrv = stdenv.mkDerivation {
            name = projectName;
            src = src.src;
            buildInputs = [
              rustc    # not from pkgs; this is an arg to singleCheckDrv
              binaryen # for wasm: some sort of optimizer
              clang_15 # for wasm: needs clang, and default (clang11) couldn't compile
              nodejs   # for wasm: need node to run tests
              pkgsi686Linux.glibc # for wasm: need stubs-32.h
              wasm-pack # for wasm: build driver
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
              cargo clippy --locked -- # -D warnings
              #cargo fmt --all -- --check
              popd

              # Do wasm-pack
              #export WASM_PACK_CACHE=$PWD/wasm-pack-cache
              #cp ${lockFileFn src} Cargo.lock
              #cargo install wasm-bindgen-cli ## FIXME this line currently does not work since we need to vendor wasm-bindgen-cli somehow, of a version that matches wasm-bindgen (which is vendored)
              #printf '\n[lib]\ncrate-type = ["cdylib", "rlib"]\n' >> Cargo.toml
              #CC=clang wasm-pack build -- --locked
              #CC=clang wasm-pack test --node -- --locked
              #mv Cargo-old.toml Cargo.toml

              # Check whether C code is consistent with upstream
              pushd secp256k1-sys
              patchShebangs ./vendor-libsecp.sh
              mkdir depend2/
              cp depend/*.patch depend/check_uint128_t.c depend2/
              SECP_VENDOR_CP_NOT_CLONE=yes \
                  SECP_VENDOR_GIT_ROOT=".." \
                  SECP_VENDOR_SECP_REPO=${libsecpSrc} \
                  SECP_VENDOR_DEPEND_DIR=./depend2/ \
                  ./vendor-libsecp.sh -f  # use -f to avoid calling git in a non-git repo

              cp depend/secp256k1-HEAD-revision.txt depend2/
              rm depend/secp256k1/*/*.orig || true # These files are weird seem to depend on `diff` weirdness
              rm depend2/secp256k1/*/*.orig || true
              diff -r depend/ depend2
              popd

              touch $out
            '';
          };
        in
        if projectName == "final-checks"
        then finalDrv
        else drv.overrideDerivation (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = projectName;
          checkPrPrNum = prNum;
          checkPrWorkspace = workspace;
          checkPrRustc = rustc;
          checkPrFeatures = builtins.toJSON features;
          checkPrSrc = builtins.toJSON src;
        });
  };
in
{
  checkPr = utils.checkPr checkData;
  checkHead = utils.checkPr (checkData // rec {
    argsMatrices = map
      (argsMtx: argsMtx // rec {
        isTip = _: true;
        src = rec {
          src = builtins.fetchGit {
            allRefs = true;
            url = jsonConfig.gitDir;
            rev = singleRev;
          };
          name = builtins.toString prNum;
          shortId = name;
          commitId = builtins.substring 0 8 name;
        };
      })
      checkData.argsMatrices;
  });
}
