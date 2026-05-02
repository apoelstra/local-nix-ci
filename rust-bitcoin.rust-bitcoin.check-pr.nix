let
  utils = import ./andrew-utils.nix { };
  lib = utils.nixpkgs.lib;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverrideWithPrev = prev: {
    # Use MSRV as a proxy for "this is an old broken version"
#    runClippy = { src, features, rustc, isMainWorkspace, isMainLockFile, msrv, ... } @ args: (prev.runClippy args) && msrv >= "1.63.0";

    rustc = { src, msrv, isMainLockFile, ... } @ args:
      if isMainLockFile
      then prev.rustc args
      else builtins.head (prev.rustc args); # MSRV only

    runFmt = false;
    releaseMode = false; # ungodly slow

    features = { src, cargoToml, workspace, needsNoStd, rustc, ... } @ args:
      if workspace == "bitcoin"
      then utils.featuresForSrc { exclude = [ "actual-serde" ]; } { inherit src cargoToml needsNoStd rustc; }
      # schemars does not work with nostd, so exclude it from
      # the standard list and test it separately.
      else if workspace == "hashes"
      then utils.featuresForSrc {
        include = [ [ "std" "schemars" ] ];
        exclude = [ "actual-arbitrary" "actual-serde" "schemars" ];
      } { inherit src cargoToml needsNoStd rustc; }
      else utils.featuresForSrc { } { inherit src cargoToml needsNoStd rustc; };

    extraTestPostRunTopLevel = { cargoToml, ... }:
    # FIXME remove this false at some point, maybe 2026-05-10
    lib.optionalString (false && cargoToml ? dependencies && cargoToml.dependencies ? "libfuzzer-sys") ''
      CHECKDIR=$(mktemp -d)
      cp -r . "$CHECKDIR"
      chmod +w -R "$CHECKDIR"
      pushd "$CHECKDIR"

      cp fuzz/Cargo.toml old-Cargo.toml
      cp .github/workflows/cron-daily-fuzz.yml old-daily-fuzz.yml

      cd fuzz/
      patchShebangs ./generate-files.sh
      sed -i 's#(cargo fuzz#(${utils.nixpkgs.cargo-fuzz}/bin/cargo-fuzz#' ./generate-files.sh
      sed -i 's/REPO_DIR=.*/REPO_DIR=../' generate-files.sh
      sed -i 's/REPO_DIR=.*/REPO_DIR=../' fuzz-util.sh
      ./generate-files.sh
      cd ..

      diff fuzz/Cargo.toml old-Cargo.toml
      diff .github/workflows/cron-daily-fuzz.yml old-daily-fuzz.yml

      popd
    '';
  };
}
