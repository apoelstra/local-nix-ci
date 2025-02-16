let
  utils = import ./andrew-utils.nix { };
  has_numeric_feature = feat_list: builtins.any (s: builtins.match "^[0-9_]+$" s != null) feat_list;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    features = { workspace, src, cargoToml, ... }:
    let
      normalFeatures = utils.featuresForSrc {} { inherit src cargoToml; };
    in
      if workspace == "corepc-node"
      then builtins.filter has_numeric_feature normalFeatures
      else normalFeatures;
   };
}
