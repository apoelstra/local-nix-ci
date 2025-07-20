let
  utils = import ./andrew-utils.nix { };
  lib = utils.nixpkgs.lib;
in import ./rust.check-pr.nix {
  fullMatrixOverride = {
    simplicityRevFile = { src, ... }: builtins.elemAt (builtins.split "\n"
      (builtins.readFile "${src.src}/simplicity-sys/depend/simplicity-HEAD-revision.txt"))
      2;
    simplicitySrc = { simplicityRevFile, ... }: builtins.fetchGit {
      allRefs = true;
      url = "https://github.com/BlockstreamResearch/simplicity/";
      rev = simplicityRevFile;
    };

    extraTestPostRun = { src, workspace, simplicitySrc, ... }:
      let
        simplicitySysToml = lib.trivial.importTOML "${src.src}/simplicity-sys/Cargo.toml";
        shortVersion = builtins.replaceStrings [ "." ] [ "_" ]
          (lib.versions.majorMinor simplicitySysToml.package.version);
      in
      # For old versions of rust-simplicity you've gotta disable this since simplicitySrc won't build
      #if workspace == "." && false
      if workspace == "."
        then ''
          # Check whether jets are consistent with upstream
          ${(import "${simplicitySrc}/default.nix" {}).haskell}/bin/GenRustJets
          set -x

          # FIXME try diffing in both old and new style; eventually just do new style/
          diff jets_ffi.rs ./simplicity-sys/src/c_jets/jets_ffi.rs || (
            chmod +w ./simplicity-sys/src/c_jets/jets_ffi.rs
            sed -i -r "s/\"rustsimplicity_${shortVersion}_c_/\"c_/" \
                "./simplicity-sys/src/c_jets/jets_ffi.rs"
            diff jets_ffi.rs ./simplicity-sys/src/c_jets/jets_ffi.rs
          )

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

          cp -r ${simplicitySrc}/C simplicityCReference
          chmod +w -R simplicityCReference
          find "simplicityCReference" \( -name "*.[ch]" -o -name '*.inc' \) -type f -print0 | xargs -0 \
              sed -i "/^#include/! s/simplicity_/rustsimplicity_${shortVersion}_/g"
          find "simplicityCReference" \( -name "*.[ch]" -o -name '*.inc' \) -type f -print0 | xargs -0 \
              sed -i "s/rustsimplicity_${shortVersion}_err/simplicity_err/g"

          # If the diff fails, attempt to diff against the unmodified source (TODO
          # eventually we should be able to just remove this).
          diff -r -x simplicity_alloc.h simplicityCReference depend/simplicity/ || \
            diff -r -x simplicity_alloc.h ${simplicitySrc}/C depend/simplicity/
        ''
      else "";
  };
}
