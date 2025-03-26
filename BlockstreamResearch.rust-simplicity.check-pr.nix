import ./rust.check-pr.nix {
  fullMatrixOverride = {
    simplicityRevFile = { src, ... }: builtins.elemAt (builtins.split "\n"
      (builtins.readFile "${src.src}/simplicity-sys/depend/simplicity-HEAD-revision.txt"))
      2;
    simplicitySrc = { simplicityRevFile, ... }: builtins.fetchGit {
      allRefs = true;
      url = "https://github.com/BlockstreamResearch/simplicity/";
      rev = simplicityRevFile;
    };

    extraTestPostRun = { workspace, simplicitySrc, ... }:
      # For old versions of rust-simplicity you've gotta disable this since simplicitySrc won't build
      #if workspace == "." && false
      if workspace == "."
        then ''
          # Check whether jets are consistent with upstream
          ${(import "${simplicitySrc}/default.nix" {}).haskell}/bin/GenRustJets
          set -x
          diff jets_ffi.rs ./simplicity-sys/src/c_jets/jets_ffi.rs
          diff jets_wrapper.rs ./simplicity-sys/src/c_jets/jets_wrapper.rs
          diff core.rs ./src/jet/init/core.rs
          diff bitcoin.rs ./src/jet/init/bitcoin.rs
          diff elements.rs ./src/jet/init/elements.rs
          rm jets_ffi.rs
          rm jets_wrapper.rs
          rm core.rs
          rm bitcoin.rs
          rm elements.rs
        ''
      else if workspace == "simplicity-sys"
        then ''
          # Check whether C code is consistent with upstream, explicitly excluding
          # simplicity_alloc.h which we need to rewrite for the Rust bindings.
          set -x
          diff -r -x simplicity_alloc.h ${simplicitySrc}/C depend/simplicity/
        ''
      else "";
  };
}
