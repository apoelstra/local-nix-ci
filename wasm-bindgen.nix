{ lib 
, rustPlatform
, fetchCrate
, nodejs
, pkg-config
, openssl_1_1
, stdenv
, runCommand
}:

rustPlatform.buildRustPackage rec {
  pname = "wasm-bindgen-cli";
  version = "0.2.69";

  src = fetchCrate {
    inherit pname version;
    sha256 = "sha256-oB6ZPgKO72BmwEF3aVxA9vMfNch9N1dy3CixMfCrmx0=";
  };  

  cargoSha256 = "sha256-U04mWfX6hRNsS3vq5L0gOG7dK0tTGdNZQnA1pX9U5qE=";

  nativeBuildInputs = [ pkg-config ];

  buildInputs = [ openssl_1_1 ]; # NOTE: will not work on darwin because I dropped some stuff --asp

  nativeCheckInputs = [ nodejs ];

  # other tests require it to be ran in the wasm-bindgen monorepo
  cargoTestFlags = [ "--test=interface-types" ];

  meta = with lib; {
    homepage = "https://rustwasm.github.io/docs/wasm-bindgen/";
    license = with licenses; [ asl20 /* or */ mit ];
    description = "Facilitating high-level interactions between wasm modules and JavaScript";
    maintainers = with maintainers; [ nitsky rizary ];
    mainProgram = "wasm-bindgen";
  };  
}
