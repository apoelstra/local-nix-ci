#!/usr/bin/env bash

set -ex

GIT_DIR="$(git rev-parse --path-format=absolute --git-common-dir)"
JSON="$GIT_DIR/../../repo.json"
OUT_DIR=/nix/var/nix/gcroots/per-user/apoelstra/ci-output

# Parse inputs
PRNUM=$1
ACK=$2
if [ "$PRNUM" == "" ]; then
	echo "Usage: $0 <prnum>"
	exit 1
elif git rev-parse --verify --quiet "pr/$PRNUM/head^{commit}"; then
	COMMIT_NAME="pr/$PRNUM/head"
	COMMIT_ID="$(git rev-parse "$COMMIT_NAME")"
	NIX_PRNUM="$PRNUM"
	TARGET=checkPr
else
	COMMIT_NAME=$PRNUM
	COMMIT_ID="$(git rev-parse "$COMMIT_NAME")"
	NIX_PRNUM="$COMMIT_ID"
	TARGET=checkHead
fi

# Do actual build
banner "Testing $PRNUM"
DRV_FILE=$(
  nix-instantiate \
	  --show-trace \
	--arg jsonConfigFile "$JSON" \
	--argstr prNum "$NIX_PRNUM" \
	-A "$TARGET" \
	"$GIT_DIR/../../check-pr.nix"
)

OUT_FILE=$(
  nix-build \
	--builders-use-substitutes \
	--no-build-output \
	--no-out-link \
	--keep-failed \
	--keep-derivations \
	--keep-outputs \
	--log-lines 100 \
	--arg jsonConfigFile "$JSON" \
	--argstr prNum "$NIX_PRNUM" \
	-A "$TARGET" \
	"$GIT_DIR/../../check-pr.nix"
)

# Add outputs to gc roots
ln -s "$DRV_FILE" "$OUT_DIR/" || echo "WARNING: failed to create gcroot link for $DRV_FILE"
ln -s "$OUT_FILE" "$OUT_DIR/" || echo "WARNING: failed to create gcroot link for $OUT_FILE"

# Add git notes to every affected commit
export GIT_NOTES_REF="refs/notes/check-pr"
TIME=$(date "+%F %H:%M:%S")

for COMMIT_DIR in "$OUT_FILE"/*; do
	git notes append "$(basename "$COMMIT_DIR")" -m "
$TIME: $DRV_FILE
$TIME: $COMMIT_DIR"
done

# Ack on github, if this is what we're supposed to do
if [ "$TARGET" == "checkPr" ]; then
	if [ "$ACK" == "ACK" ]; then
		gh pr review "$PRNUM" -a -b "ACK $(git rev-parse "pr/$PRNUM/head")"
	else
		echo "Not ACKing because second arg was not the literal text ACK."
	fi
fi

echo "Success. Added notes to $COMMIT_NAME $(git rev-parse "$COMMIT_ID")"

