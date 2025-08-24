##
## This file has is based on "tools.nix" in the crate2nix project. It is **not**
## CC0 licensed, but instead subject to the attached MIT and Apache licenses.
##
## It is from commit 236f6addfd452a48be805819e3216af79e988fd5 but modified
## by ASP to access upstream directly rather than by relative path, and to
## allow lockfile overrides.
##
## The previous version seems to have been e343a8aff8a7923941690744a604761a82b6a3d4
## with some early version of 6571b3fe3cedc24897836259e4be3f01ba3e1f01 hacked in
## ("allow deps to have workspaces", merged as #166) as well as some extra "override
## lockfile" functionality (which I could not convince kolloch was worth upstreaming).
##
## On 2024-10-18 changed to 0c9668f3018e9d51e22189c218a4de9bbc8182ae which is my PR
## 365, which fixes a problem with rust-secp-zkp that was introduced by 357.
##

#
# Some tools that might be useful in builds.
#
# Part of the "public" API of crate2nix in the sense that we will try to
# avoid breaking the API and/or mention breakages in the CHANGELOG.
#

{ pkgs ? import ./nix/nixpkgs.nix { config = { }; }
, lib ? pkgs.lib
, stdenv ? pkgs.stdenv
, strictDeprecation ? true
}:
let
  crate2nixRepo = fetchGit {
    url = "https://github.com/kolloch/crate2nix";
    ref = "master";
    allRefs = true; # only needed when using a PR
    rev = "0c9668f3018e9d51e22189c218a4de9bbc8182ae"; # pr 365
  };
  cargoNix = pkgs.callPackage "${crate2nixRepo}/crate2nix/Cargo.nix" { inherit strictDeprecation; };
  crate2nix = cargoNix.rootCrate.build;
in
rec {

  /* Returns a derivation containing the whole top-level function generated 
    by crate2nix (`Cargo.nix`) which is typically called with `pkgs.callPackage`.

    name: will be part of the derivation name
    src: the source that is needed to build the crate, usually the
    crate/workspace root directory
    cargoToml: Path to the Cargo.toml file relative to src, "Cargo.toml" by
    default.
  */
  generatedCargoNix =
    { name
    , src
    , patches ? []
    , cargoToml ? "Cargo.toml"
    , additionalCargoNixArgs ? [ ]
    , additionalCrateHashes ? internal.parseOptHashesFile
        (src + "/crate-hashes.json")
    , overrideLockFile ? null # asp
    }:
    let
      crateDir = dirOf (src + "/${cargoToml}");
      vendor = internal.vendorSupport rec {
        inherit crateDir;
        lockFiles = (internal.gatherLockFiles crateDir)
          ++ (if overrideLockFile == null then [ ] else [ overrideLockFile ]); # asp
        hashes = internal.gatherHashes (lockFiles) // additionalCrateHashes;
      };
    in
    stdenv.mkDerivation {
      name = "${name}-crate2nix";

      buildInputs = [
        # crate2nix seems not to work on rustc 1.77+; some sort of cargo-metadata
        # change is causing it to now try prefetching git dependencies where it
        # previously did not, and prefetching doesn't work because of the lack of
        # network access. Rather than untangling the series of bugs here we'll just
        # pin rust.
        #
        # This means you MUST call this using oxalica rust-overlay
        # rather than stock nixpkgs.
        pkgs.rust-bin.stable."1.76.0".default
        pkgs.jq
        crate2nix
      ];
      preferLocalBuild = true;

      inherit patches src;
      phases = [ "unpackPhase" "buildPhase" ];

      buildPhase = ''
        set -e

        mkdir -p "$out/cargo"

        export CARGO_HOME="$out/cargo"
        export HOME="$out"

        # ASP -- add overrideLockFile.
        # When overriding the lockfile, we need to convert version 4 lockfiles to version 3.
        # As of Rust 1.89 (at least) this "conversion" just means changing the version number.
        # This is necessary to run crate2nix with rustc 1.76; see comment above in `buildInputs`
        # for why we need to do this.
        ${if overrideLockFile == null then
          ""
        else ''
          cp ${overrideLockFile} ./Cargo.lock
          chmod +w ./Cargo.lock
          sed -i 's/version = 4/version = 3/' ./Cargo.lock
        ''}


        cp ${vendor.cargoConfig} $out/cargo/config

        crate_hashes="$out/crate-hashes.json"
        echo -n '${builtins.toJSON vendor.extendedHashes}' | jq > "$crate_hashes"
        # Remove last trailing newline, which crate2nix doesn't (yet) include
        truncate -s -1 "$crate_hashes"

        crate2nix_options=""
        if [ -r ./${cargoToml} ]; then
          crate2nix_options+=" -f ./${cargoToml}"
        fi

        if test -r "./crate2nix.json" ; then
          cp "./crate2nix.json" "$out/crate2nix.json"
          crate2nix_options+=" -c $out/crate2nix.json"
        fi

        if test -r "${src}/crate2nix-sources" ; then
          ln -s "${src}/crate2nix-sources" "$out/crate2nix-sources"
        fi

        set -x

        crate2nix generate \
          $crate2nix_options \
          -o "Cargo-generated.nix" \
          -h "$crate_hashes" \
          ${lib.escapeShellArgs additionalCargoNixArgs} || {
          { set +x; } 2>/dev/null
          echo "crate2nix failed." >&2
          echo "== cargo/config (BEGIN)" >&2
          sed 's/^/    /' $out/cargo/config >&2
          echo ""
          echo "== cargo/config (END)" >&2
            echo ""
            echo "== crate-hashes.json (BEGIN)" >&2
          if [ -r $crate_hashes ]; then
            sed 's/^/    /' $crate_hashes >&2
            echo ""
          else
            echo "$crate_hashes missing"
          fi
          echo "== crate-hashes.json (END)" >&2
          echo ""
          echo "== ls -la (BEGIN)" >&2
          ls -la
          echo "== ls -la (END)" >&2
          exit 3
        }
        { set +x; } 2>/dev/null

        if test -r "./crate-hashes.json" ; then
          set -x
          diff -u "./crate-hashes.json" $crate_hashes
         { set +x; } 2>/dev/null
        fi

        cp -r . $out/crate

        echo "import ./crate/Cargo-generated.nix" > $out/default.nix
      '';

    };

  # Applies the default arguments from pkgs to the generated `Cargo.nix` file.
  #
  # name: will be part of the derivation name
  # src: the source that is needed to build the crate, usually the crate/workspace root directory
  # cargoToml: Path to the Cargo.toml file relative to src, "Cargo.toml" by default.
  appliedCargoNix = { cargoToml ? "Cargo.toml", ... } @ args:
    pkgs.callPackage (generatedCargoNix args) { };

  generate =
    cargoNix.internal.deprecationWarning
      "crate2nix/tools.nix: generate deprecated since 0.7. Use generatedCargoNix instead."
      generatedCargoNix;
  generated =
    cargoNix.internal.deprecationWarning
      "crate2nix/tools.nix: generated deprecated since 0.7. Use appliedCargoNix in instead."
      appliedCargoNix;

  internal = rec {
    # Unpack sources and add a .cargo-checksum.json file to make cargo happy.
    unpacked = { sha256, src }:
      assert builtins.isString sha256;
      assert builtins.isAttrs src;

      pkgs.runCommand (lib.removeSuffix ".tar.gz" src.name) { }
        ''
          mkdir -p $out
          tar -xzf ${src} --strip-components=1 -C $out
          echo '{"package":"${sha256}","files":{}}' > $out/.cargo-checksum.json
        '';

    sourceType = { source ? null, ... } @ package:
      assert source == null || builtins.isString source;

      if source == null then
        null
      else if source == "registry+https://github.com/rust-lang/crates.io-index" then
        "crates-io"
      else if lib.hasPrefix "git+" source then
        "git"
      else
        builtins.throw "unknown source type: ${source}";

    # Extracts URL and rev from a git source URL.
    #
    # Crude, should be more robust :(
    parseGitSource = source:
      assert builtins.isString source;
      let
        withoutGitPlus = lib.removePrefix "git+" source;
        splitHash = lib.splitString "#" withoutGitPlus;
        preFragment = builtins.elemAt splitHash 0;
        fragment =
          if builtins.length splitHash >= 2
          then builtins.elemAt splitHash 1
          else null;
        splitQuestion = lib.splitString "?" preFragment;
        preQueryParams = builtins.elemAt splitQuestion 0;
        queryParamsList = lib.optionals
          (builtins.length splitQuestion >= 2)
          (lib.splitString "&" (builtins.elemAt splitQuestion 1));
        kv = s:
          let
            l = lib.splitString "=" s;
            key = builtins.elemAt l 0;
          in
          {
            # Cargo supports using the now-obsoleted "ref" key in place of
            # "branch"; see cargo-vendor source
            name =
              if key == "ref"
              then "branch"
              else key;
            value = builtins.elemAt l 1;
          };
        queryParams = builtins.listToAttrs (map kv queryParamsList);
      in
      assert builtins.length splitHash <= 2;
      assert builtins.length splitQuestion <= 2;
      queryParams // {
        url = preQueryParams;
        urlFragment = fragment;
      };

    gatherLockFiles = crateDir:
      let
        fromCrateDir =
          if builtins.pathExists (crateDir + "/Cargo.lock")
          then [ (crateDir + "/Cargo.lock") ]
          else [ ];
        fromSources =
          if builtins.pathExists (crateDir + "/crate2nix-sources")
          then
            let
              subdirsTypes = builtins.readDir (crateDir + "/crate2nix-sources");
              subdirs = builtins.attrNames subdirsTypes;
              toLockFile = subdir: (crateDir + "/crate2nix-sources/${subdir}/Cargo.lock");
            in
            builtins.map toLockFile subdirs
          else [ ];
      in
      fromCrateDir ++ fromSources;

    parseOptHashesFile = hashesFile: lib.optionalAttrs
      (builtins.pathExists hashesFile)
      (builtins.fromJSON (builtins.readFile hashesFile));

    gatherHashes = lockFiles:
      let
        hashesFiles = builtins.map
          (cargoLock: "${dirOf cargoLock}/crate-hashes.json")
          lockFiles;

        parsedFiles = builtins.map parseOptHashesFile hashesFiles;
      in
      lib.foldl (a: b: a // b) { } parsedFiles;

    vendorSupport =
      { crateDir ? ./.
      , lockFiles ? [ ]
      , hashes ? { }
      }:
      rec {
        toPackageId = { name, version, source, ... }:
          "${name} ${version} (${source})";

        locked =
          let
            parseFile = cargoLock: lib.importTOML cargoLock;
            allParsedFiles = builtins.map parseFile lockFiles;
            merge = merged: lock:
              {
                package = merged.package ++ lock.package or [ ];
                metadata = merged.metadata // lock.metadata or { };
              };
          in
          lib.foldl merge { package = [ ]; metadata = { }; } allParsedFiles;

        mkGitHash = { source, ... }@attrs:
          let
            parsed = parseGitSource source;
            src = builtins.fetchGit ({
              submodules = true;
              inherit (parsed) url;
              rev =
                if isNull parsed.urlFragment
                then parsed.rev
                else parsed.urlFragment;
            } // (if (parsed ? branch || parsed ? tag)
            then { ref = parsed.branch or "refs/tags/${parsed.tag}"; }
            else { allRefs = true; })
            );
            hash = pkgs.runCommand "hash-of-${attrs.name}" { nativeBuildInputs = [ pkgs.nix ]; } ''
              echo -n "$(nix-hash --type sha256 --base32 ${src})" > $out
            '';
          in
          rec {
            name = toPackageId attrs;
            # Fetching git submodules with builtins.fetchGit is only supported in nix > 2.3
            value = hashes.${name} or
              (if lib.versionAtLeast builtins.nixVersion "2.4"
              then builtins.readFile hash
              else builtins.throw "Checksum for ${name} not found in `hashes`");
          };

        extendedHashes = hashes
          // builtins.listToAttrs (map mkGitHash (packagesByType.git or [ ]));

        packages =
          let
            packagesWithDuplicates = assert builtins.isList locked.package; locked.package;
            packagesWithoutLocal = builtins.filter (p: p ? source) packagesWithDuplicates;
            packageById = package: { name = toPackageId package; value = package; };
            packagesById = builtins.listToAttrs (builtins.map packageById packagesWithoutLocal);
          in
          builtins.attrValues packagesById;
        packagesWithType = builtins.filter (pkg: (sourceType pkg) != null) packages;
        packagesByType = lib.groupBy sourceType packagesWithType;

        # Returns a derivation with all the transitive dependencies in
        # sub directories suitable for cargo vendoring.
        vendoredSources =
          let
            crateSources =
              builtins.map
                (
                  package:
                  let
                    fetcher = fetchers.${sourceType package};
                    source = fetcher package;
                  in
                  {
                    # We are using the store path (without the store directory)
                    # as the name of a symlink, and don't care about store
                    # store path we got that string one. It will in fact be
                    # tract in the value's string context anyways.
                    #
                    # This is needed for Nixpkgs 22.11 and beyond where the
                    # names are deduplicated with an attrset, and attrset keys
                    # are required to not have a string context.
                    name = builtins.baseNameOf (builtins.unsafeDiscardStringContext source);
                    path = source;
                  }
                )
                packagesWithType;
          in
          pkgs.linkFarm "deps" crateSources;

        cargoConfig =
          let
            gitSourceConfig =
              { source, ... }@attrs:

                assert builtins.isString source;
                let
                  parsed = parseGitSource source;
                in
                ''

                [source."${lib.removePrefix "git+" source}"]
                git = "${parsed.url}"
                ${lib.optionalString (parsed ? rev) ''rev = "${parsed.rev}"''}
                ${lib.optionalString (parsed ? tag) ''tag = "${parsed.tag}"''}
                ${lib.optionalString (parsed ? branch) ''branch = "${parsed.branch}"''}
                replace-with = "vendored-sources"
              '';
            gitSources = packagesByType."git" or [ ];
            uniqueBy = f:
              lib.foldl' (acc: e: if lib.elem (f e) (map f acc) then acc else acc ++ [ e ]) [ ];
            gitSourcesUnique = uniqueBy (c: c.source) gitSources;
            gitSourceConfigs = builtins.map gitSourceConfig gitSourcesUnique;
            gitSourceConfigsString = lib.concatStrings gitSourceConfigs;
          in
          pkgs.writeText
            "vendor-config"
            ''
              [source.crates-io]
              replace-with = "vendored-sources"
              ${gitSourceConfigsString}

              [source.vendored-sources]
              directory = "${vendoredSources}"
            '';

        # Fetchers by source type that can fetch the package source.
        fetchers = {
          "crates-io" = { name, version, source, ... } @ package:
            assert (sourceType package) == "crates-io";
            let
              packageId = toPackageId package;
              sha256 =
                package.checksum
                  or locked.metadata."checksum ${packageId}"
                  or (builtins.throw "Checksum for ${packageId} not found in Cargo.lock");
            in
            unpacked {
              src = pkgs.fetchurl {
                name = "crates-io-${name}-${version}.tar.gz";
                # https://www.pietroalbini.org/blog/downloading-crates-io/
                # Not rate-limited, CDN URL.
                url = "https://static.crates.io/crates/${name}/${name}-${version}.crate";
                inherit sha256;
              };
              inherit sha256;
            };

          "git" = { name, version, source, ... } @ package:
            assert (sourceType package) == "git";
            let
              packageId = toPackageId package;
              sha256 = extendedHashes.${packageId};
              parsed = parseGitSource source;
              src = pkgs.fetchgit {
                name = "${name}-${version}";
                inherit sha256;
                inherit (parsed) url;
                rev =
                  if isNull parsed.urlFragment
                  then parsed.rev
                  else parsed.urlFragment;
              };

              rootCargo = builtins.fromTOML (builtins.readFile "${src}/Cargo.toml");
              isWorkspace = rootCargo ? "workspace";
              isPackage = rootCargo ? "package";
              containedCrates = rootCargo.workspace.members ++ (if isPackage then [ "." ] else [ ]);

              getCrateNameFromPath = path:
                let
                  cargoTomlCrate = builtins.fromTOML (builtins.readFile "${src}/${path}/Cargo.toml");
                in
                cargoTomlCrate.package.name;

              pathToExtract =
                if isWorkspace then
                  builtins.head
                    (builtins.filter
                      (to_filter:
                        (getCrateNameFromPath to_filter) == name
                      )
                      containedCrates)
                else
                  ".";
            in
            pkgs.runCommand (lib.removeSuffix ".tar.gz" src.name) { }
              ''
                mkdir -p $out
                cp -apR ${src}/${pathToExtract}/* $out
                echo '{"package":null,"files":{}}' > $out/.cargo-checksum.json
              '';

        };
      };
  };
}
