let
  pkgs = import <nixpkgs> { };
in import ./rust.check-pr.nix {
  inherit pkgs;
  fullMatrixOverride = {
    extraTestPostRun = { workspace, ... }: if workspace == "integration_test"
      then ''
        set -x
        export BITCOIND_PATH=$BITCOIND_EXE
        export PATH=${pkgs.psmisc}/bin:${pkgs.valgrind}/bin:$PATH # for killall
        sed -i 's/cargo run/valgrind .\/target\/debug\/integration_test/' run.sh
        cat run.sh
        ./run.sh
      ''
      else "";
  };
}
