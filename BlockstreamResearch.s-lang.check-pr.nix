import ./rust.check-pr.nix {
  fullMatrixOverride = {
      runDocs = false; # not working with slang right now
  };
}
