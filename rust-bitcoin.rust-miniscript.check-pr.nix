let
  utils = import ./andrew-utils.nix { };
  lib = utils.overlaidPkgs.lib;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverrideWithPrev = prev: {
    # Stick a trace onto the major version.
    mainMajorRev = { mainCargoToml, ...} @ args:
      let rev = prev.mainMajorRev args;
      in builtins.trace "Main major rev: ${rev}" rev;

    # Miniscript 12 and lower have a required no-std feature.
    # Note: `cargoToml` is not used directly but is required by `utils.featuresForSrc`,
    #  and if we don't list it then `matrix` might not provide it.
    features = { cargoToml, rustc, mainMajorRev, ... } @ args: if builtins.compareVersions mainMajorRev "13.0" < 0
      # In 10.x we also have an "unstable" nightly-only feature.
      # FIXME also maybe I need to disable "trace" here at least on 1.41.1?
      then if builtins.compareVersions mainMajorRev "11.0" < 0 && !utils.rustcIsNightly rustc
        then builtins.map
          (l: builtins.filter (s: s != "unstable") l)
          (utils.featuresForSrc { needsNoStd = true; } args)
        else utils.featuresForSrc { needsNoStd = true; } args
      else prev.features args;

    # For Miniscript 10, 11 and 12 just disable the integration tests (I -think-
    # because they require a version of the `bitcoind` crate that won't let me
    # override the bitcoind binary, but I don't remember.)
    workspace = { mainMajorRev, ... } @ args: if mainMajorRev == "10"
      # In 10.x the fuzz tests also don't build. FIXME this seems like it should
      # be fixable.
        then builtins.filter (x: x != "fuzz" && x != "bitcoind-tests") (prev.workspace args)
      else if mainMajorRev == "11" || mainMajorRev == "12"
        then builtins.filter (x: x != "bitcoind-tests") (prev.workspace args)
        else prev.workspace args;

    # FIXME disable doctests for 10.x and lower.
    docTestCmd = { mainMajorRev, ... } @ args: if builtins.compareVersions mainMajorRev "11.0" < 0
      then ""
      else "cargo test --all-features --locked --doc";

    extraTestPostRun = { mainMajorRev, workspace, ... }:
    lib.optionalString (builtins.compareVersions mainMajorRev "13.0" >= 0 && workspace == ".") ''
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
