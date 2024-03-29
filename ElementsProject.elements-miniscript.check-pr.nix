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
  allRustcs = [
    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.58.0".default
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
  srcName = self: self.src.commitId;
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${builtins.baseNameOf self.lockFile}-${builtins.concatStringsSep "," self.features}";
  isTip = src: src == builtins.head gitCommits;
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices = [
      {
        projectName = jsonConfig.repoName;
        workspace = "elements-miniscript";
        inherit isTip srcName mtxName prNum;

        features = [
          []
          ["default"]
          ["serde"]
          ["rand"]
          ["compiler"]
          ["trace"]
          ["serde" "rand" "compiler"]
          ["serde" "rand" "compiler" "trace"]
        ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }

/*
      {
        projectName = jsonConfig.repoName;
        workspace = "bitcoind-tests";
        inherit isTip srcName mtxName prNum;

        features = [ [] ];
        rustc = builtins.head allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }
*/ # Disabled until we update version of elementsd

/*
      {
        projectName = jsonConfig.repoName;
        workspace = "descriptor-fuzz";
        inherit isTip srcName mtxName prNum;

        features = [ [] ];
        rustc = allRustcs;
        lockFile = /. + builtins.head jsonConfig.lockFiles;
        src = gitCommits;
      }
*/ # Disabled because there are link errors, I suspect related to weird fuzz stuff in rust-simplicity
    ];

    singleCheckMemo = utils.crate2nixSingleCheckMemo;

    singleCheckDrv =
      { projectName
      , prNum
      , isTip
      , workspace
      , features
      , rustc
      , lockFile
      , src
      , srcName
      , mtxName
      ,
      }:
      nixes:
        with pkgs;
        let
          bitcoinSrc = (callPackage /home/apoelstra/code/bitcoin/bitcoin/default.nix {}).bitcoin24;
          elementsSrc = (callPackage /home/apoelstra/code/ElementsProject/elements/default.nix {}).elements21;
          fuzzTargets = map
            (bin: bin.name)
            (lib.trivial.importTOML "${src.src}/fuzz/Cargo.toml").bin;
          fuzzDrv = utils.cargoFuzzDrv {
            normalDrv = drv;
            inherit projectName src lockFile nixes fuzzTargets;
          };
          drv = nixes.called.workspaceMembers.${workspace}.build.override {
            inherit features;
            runTests = true;
            testPreRun = ''
              ${rustc}/bin/rustc -V
              ${rustc}/bin/cargo -V
              echo "Tip: ${builtins.toString (isTip src)}"
              echo "PR: ${prNum}"
              echo "Commit: ${src.commitId}"
              echo "Workspace ${workspace} / Features: ${builtins.toJSON features}"
            '' + lib.optionalString (workspace == "bitcoind-tests") ''
              export BITCOIND_EXE="${bitcoinSrc}/bin/bitcoind"
              export ELEMENTSD_EXE="${elementsSrc}/bin/elementsd"
              echo "Bitcoind exe: $BITCOIND_EXE"
              echo "Elementsd exe: $ELEMENTSD_EXE"
            '';
            testPostRun =
              if isTip src && isNightly rustc
              then ''
                export PATH=$PATH:${rustc}/bin:${gcc}/bin
                export CARGO_TARGET_DIR=$PWD/target
                export CARGO_HOME=${nixes.generated}/cargo
                pushd ${nixes.generated}/crate
                #cargo clippy --locked -- -D warnings
                #cargo fmt --all -- --check
                popd
              ''
              else "";
          };
        in
        (if workspace == "descriptor-fuzz" && isNightly rustc
        then fuzzDrv
        else drv).overrideAttrs (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = projectName;
          checkPrPrNum = prNum;
          checkPrRustc = rustc;
          checkPrLockFile = lockFile;
          checkPrWorkspace = workspace;
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
        isTip = _: true;
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
