import ./rust.check-pr.nix {
  fullMatrixOverride = {
      runDocs = false; # not working with slang right now
      runClippy = false; # 2024-12-10 https://github.com/BlockstreamResearch/simfony/pull/102
  };
}
