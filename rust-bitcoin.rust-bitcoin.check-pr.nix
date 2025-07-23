let
  utils = import ./andrew-utils.nix { };
  lib = utils.nixpkgs.lib;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    #runClippy = false; # needed for 0.32 backports
    runFmt = false;
    releaseMode = false; # ungodly slow

    features = { src, cargoToml, workspace, ... }:
      if workspace == "bitcoin"
      then utils.featuresForSrc { exclude = [ "actual-serde" ]; } { inherit src cargoToml; }
      # schemars does not work with nostd, so exclude it from
      # the standard list and test it separately.
      else if workspace == "hashes"
      then utils.featuresForSrc {
        include = [ [ "std" "schemars" ] ];
        exclude = [ "actual-serde" "schemars" ];
      } { inherit src cargoToml; }
      else utils.featuresForSrc { } { inherit src cargoToml; };

    extraTestPostRun = { workspace, ... }:
    lib.optionalString (workspace == ".") ''
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
    '';
  };
}
