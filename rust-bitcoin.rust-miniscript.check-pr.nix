{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, jsonConfigFile
, prNum
}:
let
  utils = import ./andrew-utils.nix { };
  cargoHfuzz = import ./honggfuzz-rs.nix { };
  jsonConfig = lib.trivial.importJSON jsonConfigFile;
  allRustcs = [
    (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    pkgs.rust-bin.stable.latest.default
    pkgs.rust-bin.beta.latest.default
    pkgs.rust-bin.stable."1.41.0".default
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
  mtxName = self: "${self.src.shortId}-${self.rustc.name}-${self.workspace}-${builtins.baseNameOf self.lockFile}-${builtins.concatStringsSep "," self.features}";
  isTip = src: src == builtins.head gitCommits;
  checkData = rec {
    name = "${jsonConfig.repoName}-pr-${builtins.toString prNum}";

    argsMatrices =
    let baseMatrix = {
        projectName = jsonConfig.repoName;
        inherit isTip srcName mtxName prNum;

        workspace = "miniscript";
        features = [
          ["no-std"]
          ["no-std" "serde"]
          ["no-std" "rand"]
          ["no-std" "base64"]
          ["no-std" "compiler"]
          ["no-std" "trace"]
          ["no-std" "serde" "rand" "base64" "compiler"]
          ["no-std" "serde" "rand" "base64" "compiler" "trace"]
          ["std"]
          ["std" "hashbrown"] # dumb, but shouldn't fail
          ["std" "no-std"] # dumb, but shouldn't fail
          ["std" "serde"]
          ["std" "rand"]
          ["std" "base64"]
          ["std" "compiler"]
          ["std" "trace"]
          ["std" "serde" "rand" "base64" "compiler"]
          ["std" "serde" "rand" "base64" "compiler" "trace"]
        ];
        rustc = allRustcs;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      };
    in [
      baseMatrix
      (baseMatrix // {
        rustc = builtins.head allRustcs;
        features = map (x: x ++ ["unstable"]) baseMatrix.features;
      })

      {
        projectName = jsonConfig.repoName;
        inherit isTip srcName mtxName prNum;

        workspace = "miniscript-fuzz";
        features = [ [] ];
        rustc = pkgs.rust-bin.stable."1.58.0".default;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }

      {
        projectName = jsonConfig.repoName;
        inherit isTip srcName mtxName prNum;

        workspace = "bitcoind-tests";
        features = [ [] ];
        rustc = pkgs.rust-bin.stable.latest.default;
        lockFile = map (x: /. + x) jsonConfig.lockFiles;
        src = gitCommits;
      }

      # single checks
      {
        projectName = "final-checks";
        inherit isTip srcName mtxName prNum;

        workspace = "miniscript";
        features = [ [] ];
        rustc = builtins.head allRustcs;
        lockFile = /. + (builtins.head jsonConfig.lockFiles);
        src = builtins.head gitCommits;
      }
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
              echo "Bitcoind exe: $BITCOIND_EXE"
            '';
          };
          fuzzTargets = map
            (bin: bin.name)
            (lib.trivial.importTOML "${src.src}/fuzz/Cargo.toml").bin;
          singleFuzzDrv = fuzzTarget: stdenv.mkDerivation {
            name = "fuzz-${fuzzTarget}";
            src = src.src;
            buildInputs = [
              rustc
              cargoHfuzz
              # Pinned version because of breaking change in args to init_disassemble_info
              libopcodes_2_38 # for dis-asm.h and bfd.h
              libunwind       # for libunwind-ptrace.h
            ];
            phases = [ "unpackPhase" "buildPhase" ];

            buildPhase = ''
              set -x
              export CARGO_HOME=$PWD/cargo
              export HFUZZ_RUN_ARGS="--run_time 300 --exit_upon_crash"

              cargo -V
              cargo hfuzz version
              echo "Source: ${builtins.toJSON src}"
              echo "Fuzz target: ${fuzzTarget}"

              # honggfuzz rebuilds the world, including itself for some reason, and
              # it expects to be able to build itself in-place. So we need a read/write
              # copy.
              cp -r ${nixes.generated}/cargo .
              chmod -R +w cargo

              DEP_DIR=$(grep 'directory =' $CARGO_HOME/config | sed 's/directory = "\(.*\)"/\1/')
              cp -r "$DEP_DIR" vendor-copy/
              chmod +w vendor-copy/
              rm vendor-copy/*honggfuzz*
              cp -rL "$DEP_DIR/"*honggfuzz* vendor-copy/ # -L means copy soft-links as real files
              chmod -R +w vendor-copy/
              # These two lines are just a search-and-replace ... but trying to get sed to replace
              # one string full of slashes with another is an unreadable mess, so easier to just
              # erase the line completely then recreate it with echo.
              sed -i "s/directory = \".*\"//" "$CARGO_HOME/config"
              echo "directory = \"$PWD/vendor-copy\"" >> "$CARGO_HOME/config"
              cat "$CARGO_HOME/config"
              # Done crazy honggfuzz shit

              pushd fuzz/
              cargo hfuzz run "${fuzzTarget}"
              popd

              touch $out
            '';
          };
          fuzzDrv = pkgs.linkFarm
            "${projectName}-${src.shortId}-fuzz" 
            (map (x: rec {
              name = "fuzz-${path.name}";
              path = singleFuzzDrv x;
            }) fuzzTargets);
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
              cargo clippy --locked -- #-D warnings
              cargo fmt --all -- --check
              popd

              touch $out
            '';
          };
        in
        if projectName == "final-checks"
        then finalDrv
        else if workspace == "miniscript-fuzz"
        then fuzzDrv
        else drv.overrideDerivation (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = projectName;
          checkPrPrNum = prNum;
          checkPrRustc = rustc;
          checkPrLockFile = lockFile;
          checkPrFeatures = builtins.toJSON features;
          checkPrWorkspace = workspace;
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
