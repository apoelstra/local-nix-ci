{
  nixpkgs ? import <nixpkgs> {}
, honggfuzzVersion ? "0.5.55"
}:
 nixpkgs.rustPlatform.buildRustPackage rec {
  pname = "honggfuzz-rs";
  version = "0.5.55-git";

  src55 = fetchGit {
    url = "https://github.com/rust-fuzz/honggfuzz-rs";
    # 0.5.55 but with the lockfile fixed (the v0.5.55 tag has an out-of-date lockfile)
    rev = "cfdbed322d182dbc19b823e0b67e78709c28bfcb";
  };
  src56 = fetchGit {
    url = "https://github.com/rust-fuzz/honggfuzz-rs";
    ref = "refs/tags/v0.5.56";
  };

  src =
    if honggfuzzVersion == "0.5.55" then src55
    else if honggfuzzVersion == "0.5.56" then src56
    else abort "Unknown hongfuzz version {hongfuzzVersion}";
   
  cargoLock = {
    lockFile = "${src}/Cargo.lock";
  };
}
