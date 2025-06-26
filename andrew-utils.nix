# Note: From time to time nixpkgs will break things. In order to bisect the
# most straightforward way is to pull out the `nix-instantiate` command from
# test.sh (or modify test.sh itself) to set
#
#     NIX_PATH=nixpkgs=$HOME/code/NixOS/nixpkgs/master/
#
# Then, in that directory, bisect by fetching `origin` then running
#
#     git reset --hard origin/master~N
#
# Where N starts from 65536 or whatever and you bisect from there. I would NOT
# try to bisect by overriding the `nixpkgs` input here because it isn't
# consistently propagated even within this file, and even if I fixed that it
# would not be propagated into crate2nix (I suspect) or into crate2nix's generated
# IFD nixfiles (I'm pretty sure).

# Works with nixpkgs 4f3a074422623781034daf8b1a966ee556587539 and crate2nix cf034861fdc4e091fc7c5f01d6c022dc46686cf1
# The next nixpkgs commit is affected by https://github.com/NixOS/nixpkgs/issues/317323 and does not work for me.

# 4f3a074422623781034daf8b1a966ee556587539 48871 good
# e594a0fc14f457b554dce4c7bf8a8174f495a389 48870 bad  (and suspicious!)
{ nixpkgs ? import <nixpkgs> { }
, stdenv ? nixpkgs.stdenv
}:
rec {
  overlaidPkgs = import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  };
  # Re-export crate2nix tools.nix path so that I don't need to worry about its
  # exact vaule in more than one place.
  tools-nix-path = ./crate2nix/tools.nix;
  # Laziness means this is only called when used
  tools-nix = overlaidPkgs.callPackage tools-nix-path {};

  # Support for old elementsd and bitcoind. Modern nixpkgs have dropped
  # boost 1.75, and its versions of db 4.8 don't link with ancient versions
  # of gcc, and probably there are other problems. Easier to just use this
  # old nixpkgs.
  ancientNixpkgs = import (builtins.fetchTarball {
    # I believe 24.05 is the last one with boost 1.75. Though I did not try 24.11.
    # But it is definitely true that in early 2025 it's missing.
    url = "https://releases.nixos.org/nixos/24.05/nixos-24.05.7376.b134951a4c9f/nixexprs.tar.xz";
    sha256 = "sha256:1f8j7fh0nl4qmqlxn6lis8zf7dnckm6jri4rwmj0qm1qivhr58lv";
  }) {};
  elementsDotNix = nixpkgs.fetchgit {
    url = "https://github.com/roconnor-blockstream/elements-nix";
    rev = "8f55670c4d78d6ffc1dd25fbbfa61d1c95d32f59";
    outputHash = "sha256-Rtni7YRLB7aXMtrLuiKbmQR9Yx6b39Az+rLS8Jki0QE=";
  };
  # Used by bitcoind-tests in miniscript and corerpc; rather than
  # detecting whether this is needed, we just always pull it in.
  bitcoinSrc = (nixpkgs.callPackage "${elementsDotNix}/elements.nix" {
    miniupnpc = nixpkgs.callPackage "${elementsDotNix}/miniupnpc-2.2.7.nix" {};
    doFunctionalTests = false; # intermittent failures on 32-core machine
    withSource = nixpkgs.fetchgit {
      url = "https://github.com/bitcoin/bitcoin";
      rev = "v24.2";
      outputHash = "sha256-WCbh/6WfQdbCPdRQK/WAMzR42s/HxE4eM1Cf/4mrafM=";
    };
    withGCC13Patches = false;
  });
  # Similar, for rust-elements.
  elementsSrc021 = nixpkgs.callPackage "${elementsDotNix}/elements.nix" {
    # Can probably increase gcc past 13; haven't tried. But heads up that we are on borrowed
    # time here, because the glibc in modern nixpkgs is not compatible with the glibc that
    # came with gcc <13, meaning that even if we pin nixpkgs to a super old version, the
    # binary we build won't run inside the crate2nix derivation that was built with the
    # modern nixpkgs.
    #
    # To compile with gcc 13, we apply two patches which are included in elements.nix. But
    # over time we will be forced to update gcc until small patches won't cut it. We need
    # to get off of elements 0.21!
    #
    # Curiously, compiling with gcc 14 works but then I get rust-elements test failures...
    # don't care to investigate anymore.
    stdenv = ancientNixpkgs.gcc13Stdenv;
    boost = ancientNixpkgs.boost175;
    db48 = ancientNixpkgs.db48;
    miniupnpc = nixpkgs.callPackage "${elementsDotNix}/miniupnpc-2.2.7.nix" {};
    libevent = ancientNixpkgs.libevent;
    doCheck = false;
    withGCC13Patches = true;

    withSource = nixpkgs.fetchgit {
      url = "https://github.com/ElementsProject/elements";
      rev = "elements-0.21.0.2";
      outputHash = "sha256-VcfJu7svpoXGGDMfIHofqCd43eTmvGOABtFwbkb6kU0=";
    };
  };
  elementsSrc22 = nixpkgs.callPackage "${elementsDotNix}/elements.nix" {
    miniupnpc = nixpkgs.callPackage "${elementsDotNix}/miniupnpc-2.2.7.nix" {};
    withSource = nixpkgs.fetchgit {
      url = "https://github.com/ElementsProject/elements";
      rev = "elements-22.1.1";
      outputHash = "sha256-bkqDuJeDq2QwzTUP0KT6IB5wpC7LIPpLIbeO4fwkUYA=";
    };
  };
  elementsSrc = nixpkgs.callPackage "${elementsDotNix}/elements.nix" {
    #sanitizers = [ "address" ]; # cool idea but causes 60s+ timeouts in tests
    doFunctionalTests = false; # four tests are failing
    miniupnpc = nixpkgs.callPackage "${elementsDotNix}/miniupnpc-2.2.7.nix" {};
    withSource = nixpkgs.fetchgit {
      url = "https://github.com/ElementsProject/elements";
      rev = "elements-23.2.4";
      outputHash = "sha256-OTX6we88fALSz0COKqjWV62pm8qro29eHl9qdiUxNdI=";
    };
  };
  # See comment near usage for what this is for.
  rustcLdLibraryPath = "${stdenv.cc.cc.lib}/lib/";

  # Takes a `src` object (as returned from `srcFromCommit`) and determines
  # the set of rustcs to test it with.
  rustcsForSrc = { src, nightlyVersion ? null, msrvVersion ? null }:
    let
      pkgs = import <nixpkgs> {
        overlays = [
          (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
        ];
      };
      msrv = pkgs.rust-bin.fromRustupToolchain { channel = msrvVersion; };
      nightly = if src.nightlyVersion != null
        then pkgs.rust-bin.fromRustupToolchain { channel = builtins.elemAt (builtins.match "([^\r\n]+)\r?\n?" src.nightlyVersion) 0; }
        else builtins.trace "warning - no rust-version, using latest nightly" (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default));
    in [
      nightly
      pkgs.rust-bin.stable.latest.default
      # In 2025-05 this started causing problems due to a new unused_code lint getting into beta
      # very shortly after we'd addressed it on rust-bitcoin master, so already-opened PRs started
      # failing with the beta compiler.
      # Also running this takes time and I don't think once in 10+ years has this ever provided
      # value.
      #pkgs.rust-bin.beta.latest.default
      msrv
    ];

  # Determines whether a given rustc is a nightly rustc.
  rustcIsNightly = rustc: !builtins.isNull (builtins.match ".*nightly-([0-9]+-[0-9]+-[0-9]+).*" rustc.version);

  # Given a list of features
  featuresForSrc = { needsNoStd ? false, include ? [], exclude ? [] }: { src, cargoToml, ... }:
    let
      randBit = name:
        let
          seed = builtins.hashString "sha256" (src.commitId + name);
          seed0 = builtins.substring 0 1 seed;
          bit = builtins.elem seed0 [ "0" "1" "2" "3" "4" "5" "6" "7" ];
        in bit;

      lib = nixpkgs.lib;
      mapToTrue = set: builtins.mapAttrs (_: _: true) set;

      cargoFeatures = { default = true; } // (if cargoToml ? features
        then mapToTrue cargoToml.features
        else {});
      optionalDeps = if cargoToml ? dependencies
        then mapToTrue (lib.filterAttrs (_: opt: opt ? optional && opt.optional) cargoToml.dependencies)
        else {};

      allFeatures = lib.filterAttrs (name: _: ! builtins.elem name exclude) (cargoFeatures // optionalDeps);
      allFeatureNames = builtins.attrNames allFeatures;

      result =
        [
          # Randoms
          (builtins.filter (name: randBit name) allFeatureNames)
          (builtins.filter (name: randBit ("0" + name)) allFeatureNames)
          # Nothing and everything
          []
          allFeatureNames
        ]
        # Each individual feature in isolation.
        ++ (map (feat: [ feat ]) allFeatureNames)
        # Explicitly included things.
        ++ include;
    in
      # This function is supposed to make a list of lists, and because this
      # is hard to keep track of with all the maps and stuff flying around,
      # we do a bunch of assertions here.
      assert builtins.all builtins.isList include;
      assert builtins.isList exclude;
      assert builtins.all builtins.isList result;
      if needsNoStd
      then map (fs: if builtins.elem "std" fs || builtins.elem "default" fs then fs else fs ++ ["no-std"]) result
      else result;

  # Given a set with a set of list-valued attributes, explode it into
  # a list of sets with every possible combination of attributes. If
  # an attribute is a function, it is evaluated with the set of all
  # non-function attributes (which will be exploded before the function
  # ones).
  #
  # Ex: matrix { a = [1 2 3]; b = [true false]; c = "Test" }
  #
  # [ { a = 1; b = true; c = "Test" }
  #   { a = 1; b = false; c = "Test" }
  #   { a = 2; b = true; c = "Test" }
  #   ...
  #   { a = 3; b = false; c = "Test" } ]
  #
  # Despite the complexity of this function and the fact that its runtime
  # is exponential in the number of args, worst-case, it runs very quickly
  # and is probably not worth further optimization. The slowness (and "too
  # many open files" failures) come from calls to singleCheckMemo, which
  # does the crate2nix IFD).
  matrix =
    let
      pkgs = nixpkgs;
      lib = pkgs.lib;
      appendKeyVal = e: k: v: e // builtins.listToAttrs [{ name = k; value = v; }];
      addNames = currentSets: prevNames: origSet:
        let
          newSet = lib.filterAttrs (k: _: !lib.elem k prevNames) origSet;
          newKeys = builtins.attrNames newSet;
        in
        if newKeys == [ ]
        then currentSets
        else
          let
            evaluateValue = s: v: if builtins.isFunction v then v s else v;
            expandValue = v: if builtins.isList v then v else [ v ];
            valueAsList = s: k: v:
              let res = expandValue (evaluateValue s v); in
                # Comment this assertion out when disabling stuff, e.g. in rust-miniscript pre 13.x
                assert lib.assertMsg (res != []) "Key ${k} has empty list for value, zeroing out whole matrix.";
                res;
            # We want to choose the next attribute such that, if it is a function,
            # then all of its inputs have already been evaluated.
            availableKeys = builtins.filter (k:
              let
                origVal = origSet.${k};
                isFunc = builtins.isFunction origVal;
                funcArgs = if isFunc
                  then builtins.functionArgs origVal
                  else {};
              in lib.all (name: lib.elem name prevNames) (builtins.attrNames funcArgs)
              ) newKeys;
            nextKey = builtins.head availableKeys;
            nextVal = origSet.${nextKey};
            newSets = builtins.concatMap (s: map
                (v: appendKeyVal s nextKey v)
                (valueAsList s nextKey nextVal)
              ) currentSets;
          in
          addNames newSets (prevNames ++ [ nextKey ]) origSet;
    in
    addNames [{ }] [ ];

  # A bunch of "standard" matrix functions useful for Rust projects
  standardRustMatrixFns =
  let
    lib = nixpkgs.lib;
    featuresName = features: "feat-" + builtins.substring
      0 8
      (builtins.hashString "sha256" (builtins.concatStringsSep "," features));
    fullTip = { src, features, rustc, isMainWorkspace, isMainLockFile, ... }: features == [ "default" ] && rustcIsNightly rustc && src.isTip && isMainWorkspace && isMainLockFile;
  in jsonConfig: {
    projectName = jsonConfig.projectName;
    src = jsonConfig.gitCommits;
    # FIXME on the command line we are passed, as value, a path to a .nix file
    #  in the Nix store. This .nix file in theory can be evaluated from in the
    #  source directory of `src` and in theory we can do this at evaluation
    #  time and thereby avoid IFD.
    #
    # However, I am struggling to get this working, so am using the old IFD logic
    # and only using the `name` of the passed cargo.nix file (and that name I'm
    # abusing as a lockfile path).
    #
    # See block comment on crate2nixMemoGeneratedCargoNix
    cargoNix = { src, ... }: lib.mapAttrsToList
      (name: value: { inherit name; })
      (builtins.trace src.cargoNixes src.cargoNixes);

    mainCargoToml = { src, ... }: lib.trivial.importTOML "${src.src}/Cargo.toml";
    workspace = { mainCargoToml, ... }:
      if mainCargoToml ? workspace
        then mainCargoToml.workspace.members
          ++ (if mainCargoToml ? package then [ "." ] else [])
        else null;

    # If there are no include/exclude rules for the crate, you can just inherit this.
    features = featuresForSrc {};

    cargoToml = { workspace, mainCargoToml, src, ... }: if workspace == null
      then mainCargoToml
      else lib.trivial.importTOML "${src.src}/${workspace}/Cargo.toml";

    msrv = { src, ...}: if src.cargoToml ? package && src.cargoToml.package ? rust-version
        then src.cargoToml.package.rust-version
        else if src.clippyToml != null && src.clippyToml ? msrv
        then src.clippyToml.msrv
        else builtins.trace "warning - no clippy.toml, using 1.56.1 as MSRV" "1.56.1";

    rustc = { src, msrv, ... }: rustcsForSrc { inherit src; msrvVersion = msrv; };
    lockFile = { src, cargoNix, ...}: if builtins.substring 0 1 cargoNix.name == "/" then /. + cargoNix.name else "${src.src}/${cargoNix.name}";
    srcName = { src, ... }: src.commitId;
    mtxName = { src, rustc, workspace, features, cargoNix, ... }: "${src.shortId}-${rustc.name}${if isNull workspace then "" else "-" + workspace}-${cargoNix.name}-${featuresName features}${if src.isTip then "-tip" else ""}";

    isMainLockFile = { src, cargoNix, ... }: cargoNix.name == builtins.head (builtins.attrNames src.cargoNixes);
    isMainWorkspace = { mainCargoToml, workspace, ... }:
      (workspace == null || workspace == builtins.head mainCargoToml.workspace.members);

    # Clippy runs with --all-targets so we only need to run it on one workspace.
    runClippy = fullTip;
    runDocs = fullTip;
    runFmt = fullTip;
    # This more-than-doubles the build time (vs not including it, in which case
    # we default to false). So this should be inherited in crates where the total
    # runtime is otherwise really fast, but probably not worthwhile otherwise.
    releaseMode = [ false true ];
  };

  # Given a commit ID, fetch it and obtain relevant data for the CI system.
  #
  # Throughout this code, a "src" refers to the set returned by this function.
  srcFromCommit =
    { commit
    , isTip
    , gitUrl
    , cargoNixes ? {}
    }:
    assert builtins.isString commit;
    assert builtins.isBool isTip;
    assert builtins.isPath gitUrl || builtins.isString gitUrl;
    rec {
      src = builtins.fetchGit {
        url = gitUrl;
        allRefs = true;
        # nb using ref rather than rev would be a bit more flexible. Experimentally,
        # this allows using short hashes, branch or tag names, etc. But since we
        # intend only to call this function with full commit IDs, we get better
        # error messages with `rev`.
        #
        # BTW, there is a lot of discussion about Nix not supporting checkouts of
        # individual commits in https://github.com/NixOS/nix/issues/4760 and in a
        # comment in nixpkgs/build-support/fetchgit/default.nix which still exists
        # today (as of bdfccb2c88b683979b99eec8f91003a89aba7878 Jan 2025).
        rev = commit;
      };
      commitId = commit;
      shortId = builtins.substring 0 8 commit;
      inherit isTip;

      # Rust-specific stuff.
      inherit cargoNixes;
      clippyToml = if builtins.pathExists "${src}/clippy.toml" then
        nixpkgs.lib.trivial.importTOML "${src}/clippy.toml"
      else null;
      cargoToml = if builtins.pathExists "${src}/Cargo.toml" then
        nixpkgs.lib.trivial.importTOML "${src}/Cargo.toml"
      else null;
      nightlyVersion = if builtins.pathExists "${src}/nightly-version" then
        builtins.readFile "${src}/nightly-version"
      else null;
    };

  derivationName = drv:
    builtins.unsafeDiscardStringContext (builtins.baseNameOf (builtins.toString drv));

  # Wrapper around linkFarm, partially written by ChatGPT o3, which injects an
  # artificial dependency between each derivation and the next, forcing Nix to
  # evaluate them sequentially. For whatever reason the Bitcoin/Elements
  # functional tests cannot successfully run, even with -j1, when too many are
  # being run in parallel.
  #
  # This assumes all derivations are stdenvs and is unlikely to work with e.g.
  # the crate2nix stuff.
  sequentialLinkFarm =
  let
    chained = name: items:
      # foldl' that threads the “previous” wrapper through the list
      builtins.foldl' (acc: item:
        let
          prevDrv = acc.prev;         # may be null for the first item
          realDrv = item.value;
          wrapper = if prevDrv == null
          then realDrv
          else realDrv.overrideAttrs (old: {
            postUnpack = ''
              echo ${prevDrv} > .serial-depends
              ${old.postUnpack or ""}
            '';
          });
        in {
          prev = wrapper;
          list = acc.list ++ [ { name = item.name; path = wrapper; } ];
        })
        # initial accumulator
        { prev = null; list = [ ]; }
        items;
  in name: items:
    let
      chainedResult = chained name items;
    in nixpkgs.linkFarm name chainedResult.list;

  # Given a bunch of data, do a full PR check.
  #
  # name is a string that will be used as the name of the total linkFarm
  # argsMatrix should be an "argument matrix", which is an arbitrary-shaped
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
    , argsMatrix
    , singleCheckDrv
    , memoGeneratedCargoNix ? x: { name = ""; value = null; }
    , memoCalledCargoNix ? x: { name = ""; value = null; }
    , forceSequential ? false
    }:
    let
      mtxs' = matrix argsMatrix;
      mtxs = builtins.trace "Full matrix has size ${builtins.toString (builtins.length mtxs')}" mtxs';
      # This twisty memoAndLinks logic is due to roconnor. It avoids recomputing
      # memo.value (which is potentially expensive), which would be needed
      # if we first computing memoTable "normally" and then later indexed into
      # it when producing a list of links.
      generatedMemoTable = builtins.listToAttrs (map (x: x.generatedMemo) memoAndLinks);
      calledMemoTable = builtins.listToAttrs (map (x: x.calledMemo) memoAndLinks);
      memoAndLinks = map
        (mtx:
          let
            generatedMemo = memoGeneratedCargoNix mtx;
            calledMemo = memoCalledCargoNix mtx generatedMemoTable.${generatedMemo.name};
            mtxHash = builtins.hashString "sha256" (builtins.toJSON mtx);
          in {
            inherit generatedMemo calledMemo;
            link = rec {
              value = singleCheckDrv mtx generatedMemoTable.${generatedMemo.name} calledMemoTable.${calledMemo.name};
              name = "${mtx.srcName}/${mtx.mtxName}--${mtxHash}";
            };
          }
        )
        mtxs;
      toFarm = map (x: x.link) memoAndLinks;
    in if forceSequential
      # sequentialLinkFarm assumes the form of toFarm is { value = X, name = Y } rather
      # than { path = X, name = Y } as the normal linkFarm does. See next comment for why.
      then sequentialLinkFarm name toFarm
      # With the real linkFarm, providing an attrset is dramatically faster since it
      # disables goofy "in case of duplicate names make sure the last item takes priority"
      # logic. With sequentialLinkFarm we don't bother since presumably if you're doing
      # heavy derivations in sequence the overhead of linkFarm is the least of your problems,
      # and because we really do want an ordered list for sequentialLinkFarm.
      else nixpkgs.linkFarm name (builtins.listToAttrs toFarm);

  # A value of singleCheckMemo useful for Rust projects, which uses a crate2nix generated
  # Cargo.nix as the key, and the result of calling it as the value.
  #
  # Assumes that your matrix has entries projectName, prNum, rustc, lockFile, src.
  #
  # Note that EVERY INPUT TO THIS FUNCTION MUST BE ADDED TO memoName. If it is
  # not, the memoization logic will collapse all the different values for that input
  # into one.
  #
  # During evaluation, this method is by far the slowest, since it is doing an IFD of
  # a crate2nix call. A single run takes several seconds, and the number of times it
  # is run is the product of the number of possibilities for each input. So "global"
  # values like `projectName` and `prNum` are free, but for each commit in `src`,
  # it will be run twice (number of lockfiles), and if you add any more arguments, it
  # will be multiplied again.
  crate2nixMemoGeneratedCargoNix =
    { projectName
    , prNum
    , lockFile
    , src
    , patches ? []
    , ...
    }:
    let
      memoName = builtins.unsafeDiscardStringContext
        "${projectName}-generated-cargo-nix-${builtins.toString prNum}-${src.shortId}-${lockFile}";
      generatedCargoNix = tools-nix.generatedCargoNix {
        name = memoName;
        src = src.src;
        overrideLockFile = lockFile;
        inherit patches;
      };
    in
    {
      name = memoName;
      value = builtins.trace "Evaluating generated ${memoName}" generatedCargoNix;
    };

  crate2nixMemoCalledCargoNix =
    { projectName
    , prNum
    , lockFile
    , rustc
    , msrv
    , src
    , patches ? []
    , workspace
    , features
    , cargoToml
    , releaseMode ? false
    , ...
    }:
    generatedCargoNix:
    let
      cargoNixPath = "${generatedCargoNix}";
      releaseModeName = if releaseMode then "release" else "debug";
      memoName = builtins.unsafeDiscardStringContext
        "${projectName}-called-cargo-nix-${builtins.toString prNum}-${src.shortId}-${lockFile}-${rustc.version}-${releaseModeName}";
      # We are given the IFD for the crate in question as `generatedCargoNix`. But to get
      # an actual derivation, we first need to call it (setting various arguments
      # according to the matrix values), then take the `build` attribute of that,
      # which we further override.
      calledCargoNix = nixpkgs.callPackage generatedCargoNix {
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
          # Unsure where this ought to live. Arguably in nixpkgs itself.
          let
            buildRustCrate = pkgs.buildRustCrate.override {
              defaultCrateOverrides = pkgs.defaultCrateOverrides // {
                hidapi = attrs: {
                  buildInputs = [ pkgs.pkg-config pkgs.hidapi pkgs.udev ];
                };
              };
            };
          in
            if builtins.elem crate.crateName rootCrateIds
            # need to force msrv in here because of https://github.com/NixOS/nixpkgs/pull/274440#pullrequestreview-2350593987
            then (buildRustCrate (crate // { rust-version = msrv; })).override {
              preUnpack = ''
                set +x
                echo "[buildRustCrate override for crate ${crate.crateName} (root)]"
                echo 'Cargo.nix: ${cargoNixPath}'
                echo 'Project name: ${projectName}'
                echo 'PR number: ${builtins.toString prNum}'
                echo 'rustc: ${builtins.toString rustc}'
                echo 'lockFile: ${lockFile}'
                echo 'Source commit: ${builtins.toString src.commitId}'
                echo 'Source: ${builtins.toString src.src}'
                echo 'Workspace: ${if isNull workspace then "[no workspaces]" else workspace}'
                echo 'Features: ${builtins.toJSON features}'
                echo 'Patches: ${builtins.toJSON patches}'

                export CARGO_BIN_NAME="${projectName}"
                echo "CARGO_BIN_NAME: $CARGO_BIN_NAME"
              '';

              rust =
              let
                # In bash we cannot set names like CARGO_BIN_EXE_hal-simplicity because
                # the - means that this is not a valid identifier. Quoting it does not
                # help. Using eval does not help. Instead you have to use env. However,
                # env only lets you run one command; it can't modify the running shell
                # environment.
                #
                # In a shell, you can run `exec env ${envString} bash` which seems to work
                # but in a nix build this seems to just terminate the build. You can also
                # run `alias rustc="env 'stuff=stuff' rustc" but doing this here does not
                # get propagated into the mkDerivation that buildRustCrate creates.
                #
                # Instead it looks like we have to override the rustc derivation itself
                # in order to set these environment variables.
                #
                # Now, -at compile time- the binaries will show up as ./target/bin/XYZ.
                # However, when we run the tests, in a separate derivation, they will
                # be off in /nix/store/whatever/bin/XYZ, and we can't compute `whatever`
                # because it includes, among other things, a hash of the binary that
                # we're compiling right now. So ./target/bin/XYZ will be wrong and the
                # correct path is cryptographically unknowable.
                #
                # HOWEVER, in crate2nix, it turns out that we copy all the binaries into
                # target/debug, for some historical reasons. So we can access them there.
                # https://github.com/nix-community/crate2nix/blob/c027e463f25c3b335a92a4a5cc9caab4c2b814f5/crate2nix/Cargo.nix#L3316-L3321
                mainToml = nixpkgs.lib.trivial.importTOML "${src.src}/Cargo.toml";
                envString = builtins.concatStringsSep " " (map (bin:
                  let
                    var = "CARGO_BIN_EXE_${bin.name}";
                    val = "./target/debug/${bin.name}";
                  in "\"${var}=${val}\"")
                  mainToml.bin);
              in if mainToml ? bin
                then nixpkgs.writeShellScriptBin "rustc" ''
                  #!/bin/sh
                  echo exec env ${envString} ${rustc}/bin/rustc "$@"
                  exec env ${envString} ${rustc}/bin/rustc "$@"
                ''
                else rustc;
            }
            else (buildRustCrate (crate // { rust-version = msrv; })).override {
              preUnpack = ''
                set +x
                echo "[buildRustCrate override for crate ${crate.crateName} (dependency)]"
                echo 'rustc: ${builtins.toString rustc}'
              '';
              rust = rustc;
            };
        release = releaseMode;
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
      name = memoName;
      value = builtins.trace "Evaluating called ${memoName}" calledCargoNix;
    };

  # A value of singleCheckDrv useful for Rust projects. Should be used with
  # `crate2nixSingleCheckMemo`.
  crate2nixSingleCheckDrv =
    { projectName
    , prNum
    , isMainLockFile
    , isMainWorkspace
    , workspace ? null
    , mainCargoToml
    , cargoToml
    , features
    , rustc
    , lockFile
    , src
    , srcName
    , mtxName
    , clippyExtraArgs ? null
    , runClippy ? true
    , runDocs ? true
    , runFmt ? false
    , docTestCmd ? "cargo test --all-features --locked --doc"
    # We have some should_panic tests in rust-bitcoin that fail in release mode
    , releaseMode ? false
    , extraTestPostRun ? ""
    , ...
    }:
    generatedCargoNix:
    calledCargoNix:
    let
      pkgs = import <nixpkgs> {
        overlays = [ (self: super: { inherit rustc;
          buildPackages = super.buildPackages // { inherit rustc; };
}) ];
      };
      lib = pkgs.lib;
      boolString = b: if b then "true" else "false";

      crate2nixDrv = if workspace == null
        then calledCargoNix.rootCrate.build
        else calledCargoNix.workspaceMembers.${cargoToml.package.name}.build;

      drv = crate2nixDrv.override {
        inherit features;
        runTests = true;
        testPreRun = ''
          ${rustc}/bin/rustc -V
          ${rustc}/bin/cargo -V
          echo "PR: ${projectName} #${prNum}"
          echo "Commit: ${src.commitId}"
          echo "Tip: ${boolString src.isTip}"
          echo "Workspace: ${if isNull workspace then "[no workspaces]" else workspace} (main: ${boolString isMainWorkspace})"
          echo "Features: ${builtins.toJSON features}"
          echo "Lockfile: ${lockFile} (main: ${boolString isMainLockFile})"
          echo
          echo "Nightly: ${boolString (rustcIsNightly rustc)}"
          echo "Run clippy: ${boolString runClippy}"
          echo "Run docs: ${boolString runDocs}"
          echo "Release mode: ${boolString releaseMode}"

          # Always pull these in; though it is usually not needed.
          echo
          export BITCOIND_EXE="${bitcoinSrc}/bin/bitcoind"
          export ELEMENTSD_EXE="${elementsSrc}/bin/elementsd"
          echo "Bitcoind exe: $BITCOIND_EXE"
          echo "Elementsd exe: $ELEMENTSD_EXE"

          # We have "cannot find libstdc++" issues when compiling
          # rust-bitcoin with bitcoinconsensus on and rustc nightly
          # from 2024-05-22 onward (did not occur on 2024-05-04;
          # intermediate versions not tested due to rustc #124800).
          #
          # Possible culprit: https://blog.rust-lang.org/2024/05/17/enabling-rust-lld-on-linux.html
          export LD_LIBRARY_PATH=${rustcLdLibraryPath}
        '';
        testPostRun = ''
            set -x
            pwd
            export PATH=$PATH:${pkgs.gcc}/bin:${rustc}/bin:${pkgs.pkg-config}/bin
            export NIXES_GENERATED_DIR=${generatedCargoNix}/
            # FIXME these two lines properly belong to icboc -- should add some config
            # field for this stuff. (Needed for `cargo test` to run with a hidapi depenedncy)
            export PKG_CONFIG_PATH=${pkgs.udev.dev}/lib/pkgconfig
            export CFLAGS="$NIX_CFLAGS_COMPILE -isystem ${pkgs.udev.dev}/include"

            export CARGO_TARGET_DIR=$PWD/target
            pushd ${generatedCargoNix}/crate
            export CARGO_HOME=../cargo

            # We need to manually run cargo test because the runTests run will not.
            # See https://github.com/nix-community/crate2nix/issues/194
            ${docTestCmd}
          '' + lib.optionalString runClippy ''
            # Nightly clippy
            cargo clippy --all-features --all-targets --locked -- -D warnings ${if isNull clippyExtraArgs then "" else clippyExtraArgs}
          '' + lib.optionalString runDocs ''
            # Do nightly "broken links" check
            export RUSTDOCFLAGS="--cfg docsrs -D warnings -D rustdoc::broken-intra-doc-links"
            cargo doc -j1 --all-features
            # Do non-docsrs check that our docs are feature-gated correctly.
            export RUSTDOCFLAGS="-D warnings"
            cargo doc -j1 --all-features
          '' + lib.optionalString runFmt ''
            cargo fmt --all -- --check
          '' + lib.optionalString (rustcIsNightly rustc && isMainLockFile && cargoToml ? dependencies && cargoToml.dependencies ? honggfuzz) ''
            echo "Ran fuzztests (cargo-fuzz): ${fuzzHonggfuzzDrv}"
          '' + lib.optionalString (rustcIsNightly rustc && isMainLockFile && cargoToml ? dependencies && cargoToml.dependencies ? "libfuzzer-sys") ''
            echo "Ran fuzztests (honggfuzz): ${fuzzLibfuzzerDrv}"
          '' + ''
            popd
          '' + extraTestPostRun;
        };
        fuzzTargets = map
          (bin: bin.name)
          (lib.trivial.importTOML "${src.src}/fuzz/Cargo.toml").bin;
        fuzzHonggfuzzDrv = cargoFuzzHonggfuzzDrv {
          inherit cargoToml projectName src lockFile generatedCargoNix fuzzTargets;
        };
        fuzzLibfuzzerDrv = cargoFuzzLibfuzzerDrv {
          inherit cargoToml projectName src lockFile generatedCargoNix fuzzTargets;
        };
      in
        # If test derivations seem to be very slow to instantiate, uncomment
        # the following line and run test.sh piped through `ts -s` to get a
        # picture of how long each instantiation takes.
        # builtins.trace "Evaluating test derivation"
        drv.overrideDerivation (drv: {
          # Add a bunch of stuff just to make the derivation easier to grok
          checkPrProjectName = projectName;
          checkPrPrNum = prNum;
          checkPrRustc = rustc;
          checkPrLockFile = lockFile;
          checkPrFeatures = builtins.toJSON features;
          checkPrWorkspace = workspace;
          checkPrSrc = builtins.toString src.commitId;
          checkPrIsMainLockFile = boolString isMainLockFile;
          checkPrRustcIsNightly = boolString (rustcIsNightly rustc);
          checkPrRunFmt = boolString runFmt;
          checkPrRunClippy = boolString runClippy;
          checkPrRunDocs = boolString runDocs;
          checkPrIsTip = boolString src.isTip;
        });

  # Derivation that runs cargo-hfuzz on a series of targets found in fuzz/Cargo.toml.
  cargoFuzzHonggfuzzDrv = {
    projectName
  , src
  , cargoToml
  , lockFile
  , generatedCargoNix
  , fuzzTargets
  }: let
    honggfuzzVersion =
      let
        lockToml = nixpkgs.lib.importTOML lockFile;
        honggfuzz = (builtins.filter (x: x.name == "honggfuzz") lockToml.package);
      in (builtins.elemAt honggfuzz 0).version;

    singleFuzzHonggfuzzDrv = fuzzTarget: stdenv.mkDerivation {
      name = "fuzz-${fuzzTarget}";
      src = src.src;
      buildInputs = [
        overlaidPkgs.rust-bin.stable."1.64.0".default
        (import ./honggfuzz-rs.nix { inherit honggfuzzVersion; })
        # Need to use libopcodes 2.38 because of https://github.com/rust-fuzz/honggfuzz-rs/issues/68
        nixpkgs.libopcodes_2_38  # for dis-asm.h and bfd.h
        nixpkgs.libunwind   # for libunwind-ptrace.h
      ];
      phases = [ "unpackPhase" "buildPhase" ];

      buildPhase = ''
        set -x
        export CARGO_HOME=$PWD/cargo
        export HFUZZ_RUN_ARGS="--run_time 120 --threads 2 --exit_upon_crash"

        cargo -V
        cargo hfuzz version
        echo "Source: ${src.commitId}"
        echo "Fuzz target: ${fuzzTarget}"

        # copied from libffi; see also https://github.com/NixOS/nixpkgs/pull/246244#issuecomment-1701571496
        NIX_HARDENING_ENABLE=''${NIX_HARDENING_ENABLE/fortify3/}
        NIX_HARDENING_ENABLE=''${NIX_HARDENING_ENABLE/fortify/}

        # honggfuzz rebuilds the world, including itself for some reason, and
        # it expects to be able to build itself in-place. So we need a read/write
        # copy.
        cp -r ${generatedCargoNix}/cargo .
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
    fuzzHonggfuzzDrv = overlaidPkgs.linkFarm
      "${projectName}-${src.shortId}-fuzz"
      (map (x: rec {
        name = "fuzz-${path.name}";
        path = singleFuzzHonggfuzzDrv x;
      }) fuzzTargets);
    in fuzzHonggfuzzDrv;

  # Derivation that runs cargo-fuzz on a series of targets found in fuzz/Cargo.toml.
  cargoFuzzLibfuzzerDrv = {
    projectName
  , src
  , cargoToml
  , lockFile
  , generatedCargoNix
  , fuzzTargets
  }: let
    singleFuzzLibfuzzerDrv = fuzzTarget: stdenv.mkDerivation {
      name = "cargo-fuzz-${fuzzTarget}";
      src = src.src;
      buildInputs = [
        # cargo-fuzz needs a nightly compiler to run since it invokes rustc with -Z
        # Pick an arbitrary nightly; don't blindly use latest because that'll be unreliable. But
        # it's probably worth updating this pin from time to time.
        (overlaidPkgs.rust-bin.fromRustupToolchain { channel = "nightly-2025-03-20"; })
        overlaidPkgs.cargo-fuzz
      ];

      phases = [ "unpackPhase" "buildPhase" ];

      buildPhase = ''
        set -x

        # As of 65e3279c9602375037cb3aaabd3209c5b746375c cargo-fuzz is hardcoded
        # to do a mkdir of artifacts/ in its --fuzz-dir argument. It does this
        # even if you override the artifacts/ directory.
        #
        # See https://github.com/rust-fuzz/cargo-fuzz/issues/406
        #
        # This means we have to do honggfuzz-like BS to copy the entire source
        # directory somewhere we can write to.

        export CARGO_TARGET_DIR=$PWD/target
        export CARGO_HOME=${generatedCargoNix}/cargo

        mkdir "$out"
        cp -r ${generatedCargoNix}/crate "$out"
        pushd "$out/crate/fuzz"
        chmod -R +w .

        cargo -V
        cargo fuzz -V
        echo "Source: ${src.commitId}"
        echo "Fuzz target: ${fuzzTarget}"
        echo "Output directory: $out"

        cargo fuzz run "${fuzzTarget}" -- -max_total_time=60

        popd
      '';
    };
   fuzzLibFuzzerDrv = overlaidPkgs.linkFarm
      "${projectName}-${src.shortId}-fuzz"
      (map (x: rec {
        name = "fuzz-${path.name}";
        path = singleFuzzLibfuzzerDrv x;
      }) fuzzTargets);
    in fuzzLibFuzzerDrv;
}



