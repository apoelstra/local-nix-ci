{ nixpkgs ? import <nixpkgs> {}
, stdenv ? nixpkgs.stdenv
}:
rec {
  # Re-export crate2nix tools.nix path so that I don't need to worry about its
  # exact vaule in more than one place.
  tools-nix-path = ../nix-setup/crate2nix/currently-using/tools.nix;

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
  matrix = let
    pkgs = import <nixpkgs> {};
    lib = pkgs.lib;
    appendKeyVal = e: k: v: e // builtins.listToAttrs [ { name = k; value = v; } ];
    addNames = currentSets: prevNames: origSet:
      let
        newSet = builtins.removeAttrs origSet prevNames;
        newKeys = builtins.attrNames newSet;
        in if newKeys == []
          then currentSets
          else let
            nextKey = builtins.head newKeys;
            nextVal = builtins.getAttr nextKey origSet;
            newSets = if builtins.isList nextVal
              then builtins.concatLists (map (v: map (s: appendKeyVal s nextKey v) currentSets) nextVal)
              else map (s: appendKeyVal s nextKey nextVal) currentSets;
            in addNames newSets (prevNames ++ [nextKey]) origSet;
    in addNames [{}] [];

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
    bareCommits = (import (githubPrCommits { inherit gitDir prNum; }) {}).gitCommits;
  in map (commit: {
    src = builtins.fetchGit {
      url = gitUrl;
      ref = "refs/pull/${builtins.toString prNum}/head";
      rev = commit;
    };
    commitId = commit;
    shortId = builtins.substring 0 8 commit;
  }) bareCommits;

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
  checkPr = {
    name,
    argsMatrices,
    singleCheckDrv,
    singleCheckMemo ? x: { name = ""; value = null; },
  }:
  let
    mtxs = builtins.concatMap matrix argsMatrices;
    # This twisty memoAndLinks logic is due to roconnor. It avoids computing
    # memo.name (which is potentially expensive) twice, which would be needed
    # if we first computing memoTable "normally" and then later indexed into
    # it when producing a list of links.
    memoTable = builtins.listToAttrs (map (x: x.memo) memoAndLinks);
    memoAndLinks = map (mtx:
      let memo = singleCheckMemo mtx;
      in {
        inherit memo;
        link = rec {
          path = singleCheckDrv mtx memoTable.${memo.name};
          name = "${mtx.srcName mtx}/${mtx.mtxName mtx}--${derivationName path}";
        };
      }
    ) mtxs;
  in nixpkgs.linkFarm name (map (x: x.link) memoAndLinks);
}



