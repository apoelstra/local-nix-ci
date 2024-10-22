let
  utils = import ./andrew-utils.nix { };
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    #runClippy = false; # needed for 0.32 backports
    runFmt = false;

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
  };
}
