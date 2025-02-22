let
  utils = import ./andrew-utils.nix { };
  has_numeric_feature = feat_list: builtins.any (s: builtins.match "^[0-9_]+$" s != null) feat_list;
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    features = { workspace, src, cargoToml, ... }:
    let
      normalFeatures = utils.featuresForSrc {} {
        inherit src cargoToml;
        exclude = [ "download" ]; # naturally we cannot download from in Nix
      };
    in
      if workspace == "corepc-node"
      then builtins.filter has_numeric_feature normalFeatures
      else normalFeatures;

    # By default we use --all-features here, but this unfortunately enables the 'download'
    # feature which causes build.rs in `node` to fail.
    docTestCmd = { mainCargoToml, ... }: if builtins.elem "node" mainCargoToml.workspace.members
      then "cargo test --locked --doc --features 0_17_1"
      else "cargo test --locked --doc --all-features";
   };
}
