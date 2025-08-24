let
  utils = import ./andrew-utils.nix { };
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    # No clippy.toml; manually set MSRV
    rustc = { src, ... }: utils.rustcsForSrc { inherit src; msrvVersion = "1.78.0"; };
   };
}
