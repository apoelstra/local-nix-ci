# Vendored from roconnor-blockstream/elements-nix (CC0), so that the
# SimplicityHL check-pr can build the 2024 elements simplicity-branch source
# against miniupnpc 2.2.7 (current nixpkgs ships 2.3.3, whose
# UPNP_GetValidIGD signature the old source does not compile against).
# Same expression andrew-utils.nix uses for its own elements builds.

{ lib
, stdenv
, fetchFromGitHub
, cmake
}:

stdenv.mkDerivation rec {
  pname = "miniupnpc";
  version = "2.2.7";

  src = fetchFromGitHub {
    owner = "miniupnp";
    repo = "miniupnp";
    rev = "miniupnpc_${lib.replaceStrings ["."] ["_"] version}";
    hash = "sha256-cIijY1NcdF169tibfB13845UT9ZoJ/CZ+XLES9ctWTY=";
  };

  sourceRoot = "${src.name}/miniupnpc";

  nativeBuildInputs = [ cmake ];

  doCheck = !stdenv.isFreeBSD;

  makeFlags = [ "PREFIX=$(out)" ];

  postInstall = ''
    chmod +x $out/lib/libminiupnpc${stdenv.hostPlatform.extensions.sharedLibrary}

    # for some reason cmake does not install binaries and manpages
    # https://github.com/miniupnp/miniupnp/issues/637
    mkdir -p $out/bin
    cp -a upnpc-static $out/bin/upnpc
    cp -a ../external-ip.sh $out/bin/external-ip
    mkdir -p $out/share/man
    cp -a ../man3 $out/share/man
  '';

  meta = with lib; {
    homepage = "https://miniupnp.tuxfamily.org/";
    description = "Client that implements the UPnP Internet Gateway Device (IGD) specification";
    platforms = with platforms; linux ++ freebsd ++ darwin;
    license = licenses.bsd3;
    mainProgram = "upnpc";
  };
}
