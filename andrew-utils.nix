{ nixpkgs ? import <nixpkgs> {}
, stdenv ? nixpkgs.stdenv
}:
rec {
  # Re-export crate2nix tools.nix path so that I don't need to worry about its
  # exact vaule in more than one place.
  tools-nix-path = ../crate2nix/tools.nix;

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
      set -ex
      export GIT_DIR="${gitDir}"

      mkdir -p "$out"

      echo 'pkgs: { gitCommits = [' >> "$out/default.nix"
      REVS=$(git rev-list "pr/${builtins.toString prNum}/head" --not "pr/${builtins.toString prNum}/merge~")
      for rev in $REVS;
        do echo " \"$rev\"" >> "$out/default.nix"
      done

      echo ']; }' >> "$out/default.nix"
    '';
  };

  # Given a bunch of data, do a full PR check
  checkPr = {
    projectName,
    prNum,
    argsMatrix,
    checkSingleCommit,
  }:
  nixpkgs.linkFarm
    "${projectName}-pr-${builtins.toString prNum}"
    (map (mtxEntry: let
      strEntry = builtins.unsafeDiscardStringContext (builtins.toJSON mtxEntry);
      hashEntry = builtins.hashString "sha256" strEntry;
    in rec {
      path = checkSingleCommit mtxEntry;
      name = "${path.name}-${hashEntry}";
    }) (matrix argsMatrix));
}

