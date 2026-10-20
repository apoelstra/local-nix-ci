let
  pkgs = import <nixpkgs> { };
in import ./rust.check-pr.nix {
  inherit pkgs;
  fullMatrixOverride = {
    # All three are broken in this crate as of 2024-06 and were disabled
    # then. FIXME files: runClippy, runFmt, runDocs.
    runClippy = false;
    runFmt = false;
    runDocs = false;

    extraTestPostRun = { workspace, ... }: if workspace == "integration_test"
      then ''
        export PATH=${pkgs.psmisc}/bin:${pkgs.valgrind}/bin:$PATH # for killall
        sed -i 's/cargo run/valgrind .\/target\/debug\/integration_test/' run.sh
        sed -i 's/bitcoind/\"$BITCOIND_EXE\"/' run.sh
        # Disabled: the test's bitcoind-wallet interaction is broken as far
        # back as bitcoin 0.21 ("wallet verification failed" errors, not
        # transient, not Nix-related). Uncomment ./run.sh once that's fixed.
        #./run.sh
      ''
      else "";
  };
}
