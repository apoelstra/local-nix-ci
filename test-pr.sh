#!/usr/bin/env bash

set -ex

GIT_DIR="$(git rev-parse --path-format=absolute --git-common-dir)"
JSON="$GIT_DIR/../../repo.json"
OUT_DIR=/nix/var/nix/gcroots/per-user/apoelstra/ci-output
OUT_FILE="$OUT_DIR/$1--$(date "+%FT%H%M%S")"

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

banner "Testing $PRNUM"

nix-build \
	--out-link "$OUT_FILE" \
	--no-build-output \
	--keep-failed \
	--arg jsonConfigFile "$JSON" \
	--arg prNum "\"$PRNUM\"" \
	-A "$TARGET" \
	"$GIT_DIR/../../check-pr.nix"

DRV_FILE=$(
nix-instantiate \
	--arg jsonConfigFile "$JSON" \
	--arg prNum "\"$PRNUM\"" \
	--add-root "$OUT_FILE.drv" \
	-A "$TARGET" \
	"$GIT_DIR/../../check-pr.nix"
)
OUT_FILE=$(
nix-build \
	--no-build-output \
	--out-link "$OUT_FILE" \
	--arg jsonConfigFile "$JSON" \
	--arg prNum "\"$PRNUM\"" \
	-A "$TARGET" \
	"$GIT_DIR/../../check-pr.nix"
)

if [ "$TARGET" == "checkPr" ]; then
	if [ "$ACK" == "ACK" ]; then
		gh pr review "$PRNUM" -a -b "ACK $(git rev-parse "pr/$PRNUM/head")"
	else
		echo "Not ACKing because second arg was not the literal text ACK."
	fi
fi

TIME=$(date "+%F %H:%M:%S")
GIT_NOTES_REF="refs/notes/check-pr" git notes append "$COMMIT_ID" -m "
$TIME: $DRV_FILE
$TIME: $OUT_FILE"

echo "Success. Added notes to $COMMIT_ID $(git rev-parse "$COMMIT_ID")"

