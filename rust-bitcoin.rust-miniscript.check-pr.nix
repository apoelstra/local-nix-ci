let
  utils = import ./andrew-utils.nix { };
  lib = utils.overlaidPkgs.lib;
  # for old versions pr 12.x -- will also need to disable the assertion in andrew-utils.nix
  # that prevents using empty sets to zero out (part of) the matrix
  oldVersion = "none"; # 12.x 11.x 10.x
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    ${if oldVersion == "none" then null else "features"} = if oldVersion == "12.x"
      then { workspace, ... } @ args: if workspace == "bitcoind-tests" then [] else utils.featuresForSrc { needsNoStd = true; } args
      else
        let oldFeatures = { rustc, ... }: [ [ "std" ] [ "std" "compiler" ]  [ "std" "compiler" "trace" ] ]
          ++ (if builtins.isNull (builtins.match "1.41" rustc.version) then [ [ "no-std" ] [ "no-std" "compiler" "trace" ] ] else [])
          ++ (if utils.rustcIsNightly rustc then [ [ "std" "unstable" "compiler" ] [ "no-std" "unstable" "compiler" ] ] else []);
        in
          if oldVersion == "11.x" then { rustc, workspace, ... } @ args: if workspace == "bitcoind-tests" then [] else oldFeatures args
          else if oldVersion == "10.x" then { rustc, workspace, ... } @ args: if workspace == "bitcoind-tests" || workspace == "fuzz" then [] else oldFeatures args
          else abort "Unknown miniscript version ${oldVersion}";

    ${if oldVersion == "10.x" then "docTestCmd" else null} = "";

    extraTestPostRun = { workspace, ... }: lib.optionalString (workspace == ".") ''
      cp fuzz/Cargo.toml old-Cargo.toml
      # Comment out for old versions
      if [ "${oldVersion}" != "10.x" && "${oldVersion}" != "11.x" ]; then
          cp .github/workflows/cron-daily-fuzz.yml old-daily-fuzz.yml
      fi

      cd fuzz/
      patchShebangs ./generate-files.sh
      sed -i 's/REPO_DIR=.*/REPO_DIR=../' generate-files.sh
      sed -i 's/REPO_DIR=.*/REPO_DIR=../' fuzz-util.sh
      ./generate-files.sh
      cd ..

      diff fuzz/Cargo.toml old-Cargo.toml
      if [ "${oldVersion}" != "10.x" && "${oldVersion}" != "11.x" ]; then
          diff .github/workflows/cron-daily-fuzz.yml old-daily-fuzz.yml
      fi
    '';
  };
}
