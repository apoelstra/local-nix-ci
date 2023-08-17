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
  allRustcs = [
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
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${lockFileName self}-${builtins.concatStringsSep "," (map (name: builtins.substring 0 8 name) self.features)}";
  lockFileFn = map (x: (src: /. + x)) jsonConfig.lockFiles;
  isTip = src: src == builtins.head gitCommits;

  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      # Main project
      rec {
        projectName = jsonConfig.repoName;
        inherit isTip srcName mtxName prNum lockFileFn;

        features = [
          [ ]
          [ "std" ]
          [ "rand" ]
          [ "rand-std" ]
          [ "recovery" ]
          [ "lowmemory" ]
          [ "serde" ]
          [ "global-context" ]
          [ "std" "bitcoin_hashes" "rand" "rand-std" "recovery" "lowmemory" "global-context" "serde" ]
          [ "bitcoin_hashes" "rand" "recovery" "lowmemory" "global-context" "serde" ]
        ];
        rustc = allRustcs;
        src = gitCommits;
      }

      # single checks
      {
        projectName = "final-checks";
        inherit isTip srcName mtxName prNum lockFileFn;

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
          libsecpRevFile = builtins.readFile "${src.src}/secp256k1-zkp-sys/depend/secp256k1-HEAD-revision.txt";
          libsecpSrc = builtins.fetchGit {
            allRefs = true;
            url = "https://github.com/ElementsProject/secp256k1-zkp/";
            rev = builtins.elemAt (builtins.split "\n" libsecpRevFile) 2;
          };
          drv = nixes.called.rootCrate.build.override {
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
            buildInputs = [ rustc ];
            phases = [ "unpackPhase" "buildPhase" ];

            buildPhase = ''
              cargo -V
              echo "Source: ${builtins.toJSON src}"

              # Run clippy/fmt checks
              export CARGO_TARGET_DIR=$PWD/target
              export CARGO_HOME=${nixes.generated}/cargo
              pushd ${nixes.generated}/crate
              cargo clippy --locked -- -D warnings
              #cargo fmt --all -- --check
              popd

              # Check whether C code is consistent with upstream
              pushd secp256k1-zkp-sys
              patchShebangs ./vendor-libsecp.sh
              mkdir depend2/
              cp depend/*.patch depend/check_uint128_t.c depend2/
              SECP_VENDOR_CP_NOT_CLONE=yes \
                  SECP_VENDOR_GIT_ROOT=".." \
                  SECP_VENDOR_SECP_REPO=${libsecpSrc} \
                  SECP_VENDOR_DEPEND_DIR=./depend2/ \
                  ./vendor-libsecp.sh -f  # use -f to avoid calling git in a non-git repo

              cp depend/secp256k1-HEAD-revision.txt depend2/
              diff -r depend/ depend2
              popd

              touch $out
            '';
          };
        in
#FIXME need to replace a couple 'git apply' lines with 'patch'
#        if projectName == "final-checks"
#        then finalDrv
#        else
drv.overrideDerivation (drv: {
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
