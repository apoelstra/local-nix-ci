{
  nixpkgs ? import <nixpkgs> {}
}:
 nixpkgs.rustPlatform.buildRustPackage rec {
  pname = "honggfuzz-rs";
  version = "0.5.55-git";

  src = fetchGit {
    url = "https://github.com/rust-fuzz/honggfuzz-rs";
    ref = "master";
  };
  cargoLock = {
    lockFile = "${src}/Cargo.lock";
  };
}
