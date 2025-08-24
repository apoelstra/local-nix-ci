let
  utils = import ./andrew-utils.nix { };
in import ./rust.check-pr.nix {
  inherit utils;
  fullMatrixOverride = {
    # No clippy.toml; manually set MSRV
    rustc = { src, ... }: utils.rustcsForSrc { inherit src; msrvVersion = "1.78.0"; };
    # cargo test --doc fails with "error: no library targets"
    docTestCmd = "";
    # 2024-08-24 -- triggers on DisplayInner, which is used in merkle.rs, which stopped
    #  being referenced in 24b11afb3663600851f0d591fca54dc17c7e95df but continues to hang
    #  around, and I guess manages to override the dead code lint until recently
    clippyExtraArgs = "-A dead_code";
   };
}
