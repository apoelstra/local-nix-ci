let
  utils = import ./andrew-utils.nix { };
  lib = utils.overlaidPkgs.lib;
  oldFeatures = { rustc, ... }: [ [ "std" ] [ "std" "compiler" ]  [ "std" "compiler" "trace" ] ]
    ++ (if builtins.isNull (builtins.match "1.41" rustc.version) then [ [ "no-std" ] [ "no-std" "compiler" "trace" ] ] else [])
    ++ (if utils.rustcIsNightly rustc then [ [ "std" "unstable" "compiler" ] [ "no-std" "unstable" "compiler" ] ] else []);
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    features = utils.featuresForSrc { needsNoStd = true; };
# for old versions pr 11.x
#    features = oldFeatures;
    # For 10.x and below
#    rustc = [ overridePkgs.rust-bin.stable.latest.default overridePkgs.rust-bin.stable."1.41.1".default overridePkgs.rust-bin.stable."1.47.0".default overridePkgs.rust-bin.beta.latest.default overridePkgs.rust-bin.nightly."2023-06-01".default ];

    extraTestPostRun = { workspace, ... }: lib.optionalString (workspace == ".") ''
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
