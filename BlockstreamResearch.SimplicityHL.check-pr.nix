let
  pkgs = import <nixpkgs> { };
  # Elements daemon with Simplicity support, as needed by the daemon integration tests
  # (spend_utxo) added in SimplicityHL PR #415. Conveniently that PR gives us a nixfile
  # to get such an elementsd.
  simplicityElementsd = src:
    if builtins.pathExists "${src}/bitcoind-tests/elementsd-simplicity.nix"
    then pkgs.callPackage "${src}/bitcoind-tests/elementsd-simplicity.nix" { }
    else null;
  # The spend_utxo test reads its .simf programs via CARGO_MANIFEST_DIR/../examples/,
  # which requires some path hacking for crate2nix.
  srcWithExamples = commit:
    if builtins.pathExists "${commit.src}/bitcoind-tests"
    then commit // {
      src = pkgs.runCommand "simplicityhl-src-with-examples" { } ''
        mkdir -p $out
        cp -r --no-preserve=mode,ownership ${commit.src}/. $out/
        chmod -R u+w $out
        if [ -f $out/bitcoind-tests/tests/spend_utxo.rs ]; then
          substituteInPlace $out/bitcoind-tests/tests/spend_utxo.rs \
            --replace-quiet '../examples/' '${commit.src}/examples/'
        fi
      '';
    }
    else commit;
in
import ./rust.check-pr.nix {
  inherit pkgs;
  fullMatrixOverride = {
      runDocs = false; # not working with slang right now
      runClippy = false; # 2024-12-10 https://github.com/BlockstreamResearch/simfony/pull/102
  };
  fullMatrixOverrideWithPrev = prev: {
    src = map srcWithExamples prev.src;
    extraTestPreRun = { src, ... }:
      let daemon = simplicityElementsd src.src; in
      if daemon != null
      then ''
        export ELEMENTSD_EXE="${daemon}/bin/elementsd"
        echo "Elementsd exe (simplicity): $ELEMENTSD_EXE"
      ''
      else "";
  };
}
