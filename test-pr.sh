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
	TARGET=checkPr
else
	COMMIT_NAME=$PRNUM
	COMMIT_ID="$(git rev-parse "$COMMIT_NAME")"
	TARGET=checkHead
fi

# Do actual build
export NIX_SHOW_STATS=1
export NIX_SHOW_STATS_PATH=./nix-instantiate-stats.json
export NIX_PATH=nixpkgs=$HOME/code/NixOS/nixpkgs/local-ci-pin/

date
banner "Testing $PRNUM"
banner "$COMMIT_ID"

#	--trace-function-calls \
DRV_FILE=$(
  time \
  nix-instantiate \
	--arg jsonConfigFile "$JSON" \
	--argstr prNum "$PRNUM" \
	-A "$TARGET" \
	"$GIT_DIR/../../check-pr.nix"
)

echo
echo "$DRV_FILE"
echo "Instantiation stats in $NIX_SHOW_STATS_PATH"
echo
#echo "Sleeping 10s to let you copy/paste the drv file"
#sleep 10

OUT_FILE=$(
  nix-build \
	--builders-use-substitutes \
	--no-build-output \
	--no-out-link \
	--keep-failed \
	--keep-derivations \
	--show-trace \
	--keep-outputs \
	--log-lines 100 \
    "$DRV_FILE"
	--log-format internal-json -v \
	2> >(nom --json)
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
		if [ "$3" == "" ]; then
			MESSAGE="successfully ran local tests"
		else
			MESSAGE="successfully ran local tests; $3"
		fi
		gh pr review "$PRNUM" -a -b "ACK $(git rev-parse "pr/$PRNUM/head") $MESSAGE"
	else
		gh pr review "$PRNUM" -c -b "Successfully ran local tests on $(git rev-parse "pr/$PRNUM/head")."
		echo "Not ACKing because second arg was not the literal text ACK. Commented."

	fi
fi

echo "Success. Added notes to $COMMIT_NAME $(git rev-parse "$COMMIT_ID")"

