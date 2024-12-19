#!/bin/sh

set -e

if [ ! -f "$1" ]; then
    echo "Usage: $0 <main derivation>"
    exit 1;
fi

OUTPUT_DIR=$(nix-store -q "$1")
N_COMMITS=$(find "$OUTPUT_DIR"/* -prune | wc -l)

echo "$N_COMMITS commits:"
for commit in "$OUTPUT_DIR"/*; do
    echo
    basename "$commit"
    for output in "$commit"/*; do
        realout=$(readlink "$output")
        deriver=$(nix-store -qd "$realout")
        tester=$(nix log "$deriver" | cut -d' ' -f3)
        echo "| $tester"
        nix show-derivation "$deriver" | jq -Ccr ".\"$deriver\" | .env | {
            \"fmt/clippy/docs/check-api\": (.checkPrRunFmt + \"/\" + .checkPrRunClippy + \"/\" + .checkPrRunDocs + \"/\" + .checkPrRunCheckPublicApi),
            \"workspace\": .checkPrWorkspace,
            \"features\": .checkPrFeatures,
            \"rustc\": .checkPrRustc,
            \"commit\": .checkPrSrc,
            \"derivation\": \"$deriver\"
        }" | sed 's/^/  /'
        #nix show-derivation "$deriver" | jq -Ccr ".\"$deriver\" | .env"
        echo
    done
done

