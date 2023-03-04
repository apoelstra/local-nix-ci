#!/usr/bin/env bash

set -ex

GIT_DIR="$(git rev-parse --path-format=absolute --git-common-dir)"
JSON="$GIT_DIR/../../repo.json"
OUT_DIR=/nix/var/nix/gcroots/per-user/apoelstra/ci-output

# Parse inputs
PRNUM=$1
ACK=$2
if git rev-parse --verify --quiet "$1^{commit}"; then
	COMMIT_ID="$PRNUM"  # no quotes
	TARGET=checkHead
elif [ "$PRNUM" == "" ]; then
	echo "Usage: $0 <prnum>"
	exit 1
else
	COMMIT_ID="pr/$PRNUM/head"
	TARGET=checkPr
fi

# Do actual build
banner "Testing $PRNUM"
DRV_FILE=$(
  nix-instantiate \
	--arg jsonConfigFile "$JSON" \
	--arg prNum "\"$PRNUM\"" \
	-A "$TARGET" \
	"$GIT_DIR/../../check-pr.nix"
)

OUT_FILE=$(
  nix-build \
	--no-build-output \
	--no-out-link \
	--keep-failed \
	--arg jsonConfigFile "$JSON" \
	--arg prNum "\"$PRNUM\"" \
	-A "$TARGET" \
	"$GIT_DIR/../../check-pr.nix"
)

# Add outputs to gc roots
ln -sf "$DRV_FILE" "$OUT_DIR/"
ln -sf "$OUT_FILE" "$OUT_DIR/"

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

echo "Success. Added notes to $COMMIT_ID $(git rev-parse "$COMMIT_ID")"

