let
  pkgs = import <nixpkgs> { };
  # Elements daemon with Simplicity support, as needed by the daemon integration tests
  # (spend_utxo) added in SimplicityHL PR #415. Conveniently that PR gives us a nixfile
  # to get such an elementsd. Less conveniently, it doesn't work with latest nixpkgs
  # and needs a bit of CXXFLAG hackery. (See andrew-utils.nix for similar stuff with
  # stock bitcoind/elementsd).
  simplicityElementsd = src:
    let
      # Same 2.2.7 pin that andrew-utils' elements builds use
      miniupnpc227 = pkgs.callPackage ./miniupnpc-2.2.7.nix { };
      daemon = if builtins.pathExists "${src}/bitcoind-tests/elementsd-simplicity.nix"
        then (pkgs.extend (_: prev: {
          elementsd = prev.elementsd.override { miniupnpc = miniupnpc227; };
        })).callPackage "${src}/bitcoind-tests/elementsd-simplicity.nix" { }
        else null;
    in if daemon != null
      then daemon.overrideAttrs (_: {
        CXXFLAGS = "-include cstdint";
      })
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
          mkdir -p $out/bitcoind-tests/examples
          cp -r $out/examples/. $out/bitcoind-tests/examples/
          substituteInPlace $out/bitcoind-tests/tests/spend_utxo.rs \
            --replace-quiet '../examples/' 'examples/'
        fi
      '';
    }
    else commit;
in
import ./rust.check-pr.nix {
  inherit pkgs;
  fullMatrixOverride = {
      runDocs = false; # not working with slang right now
  };
  fullMatrixOverrideWithPrev = prev: {
    src = map srcWithExamples prev.src;
    extraTestPreRun = { src, ... }:
      let daemon = simplicityElementsd src.src; in
      if daemon != null
      then ''
        export ELEMENTSD_EXE="${daemon}/bin/elementsd"
        # The bitcoind crate backing the elementsd harness creates its datadir under
        # TEMPDIR_ROOT, which defaults to /build on the (slow) build partition. Its
        # jsonrpc client has a hard-coded 15s RPC timeout; under parallel-build load
        # wallet creation can exceed it, after which the crate misreads the timeout as
        # "wallet already exists" and the fallback loadwallet fails with "Data is not
        # in recognized format" / "already loaded". Point the datadir at the sandbox's
        # tmpfs so wallet creation cannot be disk-starved.
        # tempfile uses create_dir, not create_dir_all: the root must pre-exist.
        # Concurrent matrix builds may share the root (PID namespaces); the crate
        # creates randomly-named datadirs inside it, so sharing is harmless.
        export TEMPDIR_ROOT="/dev/shm/local-ci-elementsd-$$"
        mkdir -p "$TEMPDIR_ROOT"
        echo "Elementsd exe (simplicity): $ELEMENTSD_EXE"
        echo "TEMPDIR_ROOT: $TEMPDIR_ROOT"
      ''
      else "";
  };
}
