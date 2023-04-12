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

        workspace = "descriptor-fuzz";
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
          fuzzDrv = stdenv.mkDerivation {
            name = projectName;
            src = src.src;
            buildInputs = [
              rustc
              # Pinned version because of breaking change in args to init_disassemble_info
              libopcodes_2_38 # for dis-asm.h and bfd.h
              libunwind       # for libunwind-ptrace.h
            ];
            phases = [ "unpackPhase" "buildPhase" ];

            buildPhase = ''
              cargo -V
              echo "Source: ${builtins.toJSON src}"

              cp -r ${nixes.generated}/cargo ${nixes.generated}/crate .
              chmod -R +w cargo crate

              # nb cargo-hfuzz cannot handle this being an absolute path; FIXME should file bug
              export CARGO_TARGET_DIR=target
              export CARGO_HOME=$PWD/cargo
              export HFUZZ_BUILD_ARGS="--features honggfuzz_fuzz"
              export HFUZZ_RUN_ARGS="--run_time 300 --exit_upon_crash"

              # honggfuzz rebuilds the world, and expects to be able to build itself
              # in-place, meaning that we can't build it from the read-only source
              # directory. So just copy everything here.
              set -x
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

              pushd crate/fuzz
              cargo test --locked

              cargo install honggfuzz --no-default-features
              for target in $(grep 'name =' Cargo.toml | grep -v "${workspace}" | sed 's/name = "\(.*\)"/\1/'); do
                  time cargo hfuzz run "$target"
              done
              cargo fmt --all -- --check
              popd

              touch $out
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
              cargo fmt --all -- --check
              popd

              touch $out
            '';
          };
        in
        if projectName == "final-checks"
        then finalDrv
        else if workspace == "descriptor-fuzz"
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
