let
  utils = import ./andrew-utils.nix { };
  lib = utils.nixpkgs.lib;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverrideWithPrev = prev: {
    # Use MSRV as a proxy for "this is an old broken version"
    runClippy = { src, features, rustc, isMainWorkspace, isMainLockFile, msrv, ... } @ args: (prev.runClippy args) && msrv >= "1.63.0";

    # We don't have a direct way to say "run once per PR", but the default value for
    # `runClippy` happens to be "run once per PR", so use that for fuzzing. On
    # rust-bitcoin it's more than doubling the runtime to be fuzzing on each commit.
    runFuzz = prev.runClippy;

    runFmt = false;
    releaseMode = false; # ungodly slow

    features = { src, cargoToml, workspace, needsNoStd, ... } @ args:
      if workspace == "bitcoin"
      then utils.featuresForSrc { exclude = [ "actual-serde" ]; } { inherit src cargoToml needsNoStd; }
      # schemars does not work with nostd, so exclude it from
      # the standard list and test it separately.
      else if workspace == "hashes"
      then utils.featuresForSrc {
        include = [ [ "std" "schemars" ] ];
        exclude = [ "actual-arbitrary" "actual-serde" "schemars" ];
      } { inherit src cargoToml needsNoStd; }
      else utils.featuresForSrc { } { inherit src cargoToml needsNoStd; };

    extraTestPostRunTopLevel = { workspace, needsNoStd, msrv, ... }:
    # FIXME remove the msrv requirement by fixing generate-files.sh on backport branches;
    # remove the "false" by just waiting a week or two for all PRs to be based on fixed commit.
    lib.optionalString (false && msrv >= "1.63.0" && ! needsNoStd && workspace == "bitcoin") ''
      CHECKDIR=$(mktemp -d)
      cp -r . "$CHECKDIR"
      chmod +w -R "$CHECKDIR"
      pushd "$CHECKDIR"

      cp fuzz/Cargo.toml old-Cargo.toml
      cp .github/workflows/cron-daily-fuzz.yml old-daily-fuzz.yml

      cd fuzz/
      patchShebangs ./generate-files.sh
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
