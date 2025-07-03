let
  utils = import ./andrew-utils.nix { };
  lib = utils.overlaidPkgs.lib;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverrideWithPrev = prev: {
    # Miniscript 12 and lower have a required no-std feature.
    # Note: `cargoToml` is not used directly but is required by `utils.featuresForSrc`,
    #  and if we don't list it then `matrix` might not provide it.
    features = { cargoToml, rustc, mainMajorRev, ... } @ args: if mainMajorRev < "13"
      # In 10.x we also have an "unstable" nightly-only feature.
      # FIXME also maybe I need to disable "trace" here at least on 1.41.1?
      then if mainMajorRev < "11" && !utils.rustcIsNightly rustc
        then builtins.map
          (l: builtins.filter (s: s != "unstable") l)
          (utils.featuresForSrc { needsNoStd = true; } args)
        else utils.featuresForSrc { needsNoStd = true; } args
      else prev.features args;

    # For Miniscript 12 and lower just disable the integration tests (I -think-
    # because they require a version of the `bitcoind` crate that won't let me
    # override the bitcoind binary, but I don't remember.)
    workspace = { mainMajorRev, ... } @ args: if mainMajorRev < "13"
      # In 10.x the fuzz tests also don't build. FIXME this seems like it should
      # be fixable.
      then if mainMajorRev < "11"
        then builtins.filter (x: x != "fuzz" && x != "bitcoind-tests") (prev.workspace args)
        else builtins.filter (x: x != "bitcoind-tests") (prev.workspace args)
      else prev.workspace args;

    # FIXME disable doctests for 10.x and lower.
    docTestCmd = { mainMajorRev, ... } @ args: if mainMajorRev < "11"
      then ""
      else "cargo test --all-features --locked --doc";

    extraTestPostRun = { mainMajorRev, workspace, ... }:
    lib.optionalString (mainMajorRev >= "13" && workspace == ".") ''
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
