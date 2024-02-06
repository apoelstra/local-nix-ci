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
  srcName = self: self.src.commitId;
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${builtins.baseNameOf self.lockFile}-${builtins.concatStringsSep "," self.features}";
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      {
        projectName = jsonConfig.repoName;
        inherit srcName mtxName prNum;

        isTip = false;

        features = [
          [ ]
          [ "alloc" ]
          [ "default" ]
        ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }

      # Only tip
      {
        projectName = jsonConfig.repoName;
        inherit srcName prNum;

        isTip = true;

        features = [ [ "default" ] ];
        rustc = nightlyRustc;
        lockFile = /. + builtins.head jsonConfig.lockFiles;
        src = builtins.head gitCommits;

        mtxName = self: (mtxName self) + "-tip";
      }
    ];

    singleCheckMemo = utils.crate2nixSingleCheckMemo;

    singleCheckDrv =
      { projectName
      , prNum
      , isTip
      , features
      , rustc
      , lockFile
      , src
      , srcName
      , mtxName
      ,
      }:
      nixes:
        nixes.called.rootCrate.build.override {
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
          testPostRun =
            if isTip
            then ''
              # These two export lines are needed for `cargo test --doc` (or more generally,
              # cargo invocations that compile code). The first line allows rust-secp256k1
              # to be built (otherwise fails "can't find cc"). The CARGO_HOME line avoids
              # a "cannot find dependency mutagen" error, for reasons I don't really understand,
              # but I must've understood at some point in the past, because I had this solution
              # in rust-simplicity.
              export PATH=$PATH:${rustc}/bin:${pkgs.gcc}/bin
              export CARGO_HOME=${nixes.generated}/cargo # allows cargo test --doc to work
              cargo fmt --check
              cargo test --doc
              cargo clippy
            ''
            else "";
        };
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
