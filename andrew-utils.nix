{ nixpkgs ? import <nixpkgs> { }
, stdenv ? nixpkgs.stdenv
}:
rec {
  # Re-export crate2nix tools.nix path so that I don't need to worry about its
  # exact vaule in more than one place.
  tools-nix-path = ./crate2nix/tools.nix;
  # Laziness means this is only called when used
  tools-nix = nixpkgs.callPackage tools-nix-path { };

  # Given a set with a set of list-valued attributes, explode it into
  # a list of sets with every possible combination of attributes. If
  # any attribute is a non-list it is treated as a single-valued list.
  #
  # Ex: matrix { a = [1 2 3]; b = [true false]; c = "Test" }
  #
  # [ { a = 1; b = true; c = "Test" }
  #   { a = 1; b = false; c = "Test" }
  #   { a = 2; b = true; c = "Test" }
  #   ...
  #   { a = 3; b = false; c = "Test" } ]
  matrix =
    let
      pkgs = import <nixpkgs> { };
      lib = pkgs.lib;
      appendKeyVal = e: k: v: e // builtins.listToAttrs [{ name = k; value = v; }];
      addNames = currentSets: prevNames: origSet:
        let
          newSet = builtins.removeAttrs origSet prevNames;
          newKeys = builtins.attrNames newSet;
        in
        if newKeys == [ ]
        then currentSets
        else
          let
            nextKey = builtins.head newKeys;
            nextVal = builtins.getAttr nextKey origSet;
            newSets =
              if builtins.isList nextVal
              then builtins.concatLists (map (v: map (s: appendKeyVal s nextKey v) currentSets) nextVal)
              else map (s: appendKeyVal s nextKey nextVal) currentSets;
          in
          addNames newSets (prevNames ++ [ nextKey ]) origSet;
    in
    addNames [{ }] [ ];

  # Given a git directory (the .git directory) and a PR number, obtain a list of
  # commits corresponding to that PR. Assumes that the refs pr/${prNum}/head and
  # pr/${prNum}/merge both exist.
  #
  # Ex.
  # let
  # pr1207 = import (andrew.githubPrCommits {
  #   gitDir = ../../bitcoin/secp256k1/master/.git;
  #   prNum = 1207;
  # }) {};
  # in
  #   pr1207.gitCommits
  #
  githubPrCommits =
    { gitDir
    , prNum
    }:
    stdenv.mkDerivation {
      name = "get-git-commits";
      buildInputs = [ nixpkgs.git ];

      preferLocalBuild = true;
      phases = [ "buildPhase" ];

      buildPhase = ''
        set -e
        export GIT_DIR="${gitDir}"

        MERGE_COMMIT="pr/${builtins.toString prNum}/merge"
        if ! git rev-parse --verify --quiet "$MERGE_COMMIT^{commit}"; then
          echo "Merge commit $MERGE_COMMIT not found in git dir $GIT_DIR."
          exit 1
        fi

        HEAD_COMMIT="pr/${builtins.toString prNum}/head"
        if ! git rev-parse --verify --quiet "$HEAD_COMMIT^{commit}"; then
          echo "Merge commit $HEAD_COMMIT not found in git dir $GIT_DIR."
          exit 1
        fi

        mkdir -p "$out"

        echo 'pkgs: { gitCommits = [' >> "$out/default.nix"
        REVS=$(git rev-list "$HEAD_COMMIT" --not "$MERGE_COMMIT~")
        for rev in $REVS;
          do echo " \"$rev\"" >> "$out/default.nix"
        done

        echo ']; }' >> "$out/default.nix"
      '';
    };

  # Wrapper of githubPrCommits that actually calls the derivation to obtain the list of
  # git commits, then calls fetchGit on everything in the list
  #
  # If gitUrl is provided, it is used to fetch the actual commits. This is provided
  # to assist with remote building: githubPrCommits is always run locally, so it is
  # fastest to provide it with a local git checkout via gitDir. But this just gets
  # a list of commit IDs. To fetch the actual commits, which will happen remotely,
  # it may be faster to provide a github URL (so the remote machine can directly
  # connect to github rather than copying over the local derivation)
  githubPrSrcs =
    { gitDir
    , prNum
    , gitUrl ? gitDir
    }:
    let
      bareCommits = (import (githubPrCommits { inherit gitDir prNum; }) { }).gitCommits;
    in
    map
      (commit: {
        src = builtins.fetchGit {
          url = gitUrl;
          ref = "refs/pull/${builtins.toString prNum}/head";
          rev = commit;
        };
        commitId = commit;
        shortId = builtins.substring 0 8 commit;
      })
      bareCommits;

  derivationName = drv:
    builtins.unsafeDiscardStringContext (builtins.baseNameOf (builtins.toString drv));

  # Given a bunch of data, do a full PR check.
  #
  # name is a string that will be used as the name of the total linkFarm
  # argsMatrices should be a list of "argument matrices", which are arbitrary-shaped
  #  sets, which will be exploded so that any list-typed fields will be turned into
  #  multiple sets, each of which has a different element from the list in place of
  #  that field. See documentation for 'matrix' for more information.
  #
  #  THERE ARE TWO REQUIRED FIELD, srcName and mtxNames. These should be functions
  #  which take the matrix itself and output a string. The srcName string is used
  #  to split the linkFarm links into separate directories, while the mtxName
  #  string goes into the derivation name and can be anything.
  #
  # singleCheck is a function which takes:
  #  1. A matrix entry
  #  2. A value from singleCheckMemo, or null if singleCheckMemo is not provided.
  #
  # singleCheckMemo is a bit of a weird function. If provided, it takes a matrix
  # entry and returns a { name, value } pair where:
  #  1. name is a string uniquely specifying the inputs needed to compute "value"
  #     from the matrix entry.
  #  2. value is some arbitrary computation.
  #
  # The idea behind singleCheckMemo is that there are often heavy computations
  # which depend only on a subset of the matrix data (e.g. calling a Cargo.nix
  # derivation in a crate2nix-based rust build, which needs only the source
  # code and Cargo.lock file). Then we can feed the output of this function into
  # builtins.listToAttrs, and nix's lazy field mean that if a given name appears
  # multiple times, the value will only be computed for the final one (i.e. the
  # one that gets accessed).
  #
  # This has the same complexity as "lookup the value if it exists, otherwise do
  # an expensive computation and cache it", though the actual mechanics are quite
  # different.
  checkPr =
    { name
    , argsMatrices
    , singleCheckDrv
    , singleCheckMemo ? x: { name = ""; value = null; }
    ,
    }:
    let
      mtxs = builtins.concatMap matrix argsMatrices;
      # This twisty memoAndLinks logic is due to roconnor. It avoids computing
      # memo.name (which is potentially expensive) twice, which would be needed
      # if we first computing memoTable "normally" and then later indexed into
      # it when producing a list of links.
      memoTable = builtins.listToAttrs (map (x: x.memo) memoAndLinks);
      memoAndLinks = map
        (mtx:
          let memo = singleCheckMemo mtx;
          in {
            inherit memo;
            link = rec {
              path = singleCheckDrv mtx memoTable.${memo.name};
              name = "${mtx.srcName mtx}/${mtx.mtxName mtx}--${derivationName path}";
            };
          }
        )
        mtxs;
    in
    nixpkgs.linkFarm name (map (x: x.link) memoAndLinks);

  # A value of singleCheckMemo useful for Rust projects, which uses a crate2nix generated
  # Cargo.nix as the key, and the result of calling it as the value.
  #
  # Assumes that your matrix has entries projectName, prNum, rustc, lockFile, src.
  #
  # Note that EVERY INPUT TO THIS FUNCTION MUST BE ADDED TO generatedCargoNix. If it is
  # not, the memoization logic will collapse all the different values for that input
  # into one.
  crate2nixSingleCheckMemo =
    { projectName
    , prNum
    , rustc
    , lockFile
    , src
    , ...
    }:
    let
      overlaidPkgs = import <nixpkgs> {
        overlays = [ (self: super: {
          inherit rustc;
        }) ];
      };
      generatedCargoNix = tools-nix.generatedCargoNix {
        name = "${projectName}-generated-cargo-nix-${builtins.toString prNum}-${src.shortId}-${builtins.toString rustc}";
        src = src.src;
        overrideLockFile = lockFile;
      };
      calledCargoNix = overlaidPkgs.callPackage generatedCargoNix {
        # For dependencies we want to simply use the stock `pkgs.buildRustCrate`. But for
        # the actual crates we're testing, it is nice to modify the derivation to include
        # A bunch of metadata about the run. Annoyingly, there isn't any way to tell what
        # a "root crate" exposed by crate2nix, so we have to sorta hack it by assembling
        # a `rootCrateIds` list and checking membership.
        #
        # Ideally we could at least check whether `crate.crateName` matches the specific
        # workspace under test, but that is yet-undetermined and right now we're in a
        # `callPackage` call so we can't even use laziness to refer to yet-undetermined
        # values.
        buildRustCrateForPkgs = pkgs: crate:
          if builtins.elem crate.crateName rootCrateIds
          then (pkgs.buildRustCrate crate).override {
            preUnpack = ''
              set +x
              echo "Project name: ${projectName}"
              echo "PR number: ${builtins.toString prNum}"
              echo "rustc: ${builtins.toString rustc}"
              echo "lockFile: ${lockFile}"
              echo "Source: ${builtins.toJSON src}"
            '';
          }
          else pkgs.buildRustCrate crate;
        # We have some should_panic tests in rust-bitcoin that fail in release mode
        release = false;
      };
      rootCrateIds =
        (if calledCargoNix ? rootCrate
         then [ calledCargoNix.rootCrate.packageId ]
         else []) ++
        (if calledCargoNix ? workspaceMembers
         then map (p: p.packageId) (builtins.attrValues calledCargoNix.workspaceMembers)
         else []);
        
    in
    {
      name = builtins.unsafeDiscardStringContext (builtins.toString generatedCargoNix);
      value = {
        generated = generatedCargoNix;
        called = calledCargoNix;
      };
    };

  cargoFuzzDrv = {
    normalDrv
  , projectName
  , src
  , lockFile
  , nixes
  , fuzzTargets
  }: let
    overlaidPkgs = import <nixpkgs> {
      overlays = [
        (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
      ];
    };
    singleFuzzDrv = fuzzTarget: stdenv.mkDerivation {
      name = "fuzz-${fuzzTarget}";
      src = src.src;
      buildInputs = [
        overlaidPkgs.rust-bin.stable."1.64.0".default
        (import ./honggfuzz-rs.nix { })
        # Pinned version because of breaking change in args to init_disassemble_info
        nixpkgs.libopcodes_2_38 # for dis-asm.h and bfd.h
        nixpkgs.libunwind       # for libunwind-ptrace.h
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

        # If we have git dependencies, we need a lockfile, or else we get an error of the
        # form "cannot use a vendored dependency without an existing lockfile". This is a
        # limitation in cargo 1.58 (and probably later) which they apparently decided to
        # fob off on us users.
        cp ${lockFile} Cargo.lock
        pushd fuzz/
        cargo hfuzz run "${fuzzTarget}"
        popd

        touch $out
      '';
    };
    fuzzDrv = overlaidPkgs.linkFarm
      "${projectName}-${src.shortId}-fuzz" 
      ((map (x: rec {
        name = "fuzz-${path.name}";
        path = singleFuzzDrv x;
      }) fuzzTargets) ++ [{
        name = "fuzz-normal-tests";
        path = normalDrv;
      }]);
    in fuzzDrv;
}



