#!/usr/bin/env bash

set -euo pipefail

# This script creates a synthetic merge commit with detailed description
# Based on the queue_merge logic from sqlite/local-ci.sh

command -v gh >/dev/null 2>&1 || { echo "gh is required but not installed. Aborting." >&2; exit 1; }
command -v git >/dev/null 2>&1 || { echo "git is required but not installed. Aborting." >&2; exit 1; }
command -v jj >/dev/null 2>&1 || { echo "jj is required but not installed. Aborting." >&2; exit 1; }

if [ $# -lt 1 ]; then
    echo "Usage: $0 <pr_number> [--requeue]" >&2
    exit 1
fi

pr_num="$1"
requeue_flag=""
if [ "${2:-}" = "--requeue" ]; then
    requeue_flag="--yes"
fi

# Validate PR number
case $pr_num in
    *[!0-9]*)
        echo "PR number must be a number, not $pr_num" >&2
        exit 1
        ;;
esac

# Get the path to compute_merge_description.py
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_CI_PATH="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPUTE_MERGE_DESC="$LOCAL_CI_PATH/sqlite/compute_merge_description.py"

if [ ! -f "$COMPUTE_MERGE_DESC" ]; then
    echo "Error: compute_merge_description.py not found at $COMPUTE_MERGE_DESC" >&2
    exit 1
fi

# First fetch the current Github refs (this is also done in compute_merge_description.py
# and in the bitcoin-maintainer-tools github-merge.py; it modifies the repo but in a way
# that has never caused me problems in the years I've been using github-merge.py).
#
# Because it's weird and non-obvious, let me highlight: this creates a synthetic pr/X/base
# ref, alongside pr/X/head and pr/X/merge (which are created by github) which points to
# the "base ref" as obtained from Github by querying with gh.
base_ref=$(gh pr view "$pr_num" --json baseRefName --jq '.baseRefName')
remote=origin
git fetch -q "$remote" "+refs/pull/$pr_num/*:refs/heads/pull/$pr_num/*"
git fetch -q "$remote" "+refs/heads/$base_ref:refs/heads/pull/$pr_num/base"

# Then create a merge commit (no signature, no description, just a merge)
jj --config signing.behavior=drop new --no-edit -r "pull/$pr_num/base" -r "pull/$pr_num/head"

# (Racily) obtain the change ID of the commit we just made. It appears that `jj new`
# cannot be made to just output the commit or change ID that it just created in a
# machine-readable format. ChatGPT suggests parsing the human-readable output but
# that seems likely to break, so instead I'm just gonna do the racy thing. Ultimately
# this doesn't matter: we expect the resulting commit to be empty every time, and we
# ask the user to sign this empty commit out-of-band, and so if we get the "wrong"
# one then the worst case we'll have vandalized the description of an empty change.
local_merge_change_id=$(jj log --no-pager --no-graph -T change_id -r "latest(pull/$pr_num/head+ & pull/$pr_num/base+)")
local_merge_change_id="${local_merge_change_id:0:12}" # truncate to 12 chars

# If it conflicts, bail out
if ! jj log --quiet -r "$local_merge_change_id & ~conflicts()" > /dev/null; then
    echo "Failed to create merge commit for PR $pr_num: conflict in merge change $local_merge_change_id" >&2
    exit 1
fi

# Obtain its description and do initial sanity checks. If anything looks funny about
# the PR (e.g. having @s in the commit message) the user will be given an opportunity
# to bail here.
description=$("$COMPUTE_MERGE_DESC" $requeue_flag -c "$local_merge_change_id" "$pr_num")

# Copy the tree hash out of the description to avoid computing it twice
tree_hash=$(echo "$description" | grep "^Tree-SHA512: " | cut -d' ' -f2)

# Get the base commit ID
base_commit=$(git rev-parse "pull/$pr_num/base")

# If we made it this far, the PR looks ok (or at least, the users says keep going).
# Add the initial message.
jj describe --quiet -r "$local_merge_change_id" -m "$description"
merge_commit=$(jj log --no-graph -r "$local_merge_change_id" -T commit_id)

echo "Created merge commit for PR $pr_num:"
echo "  Change ID: $local_merge_change_id"
echo "  Commit ID: $merge_commit"
echo "  Tree hash: $tree_hash"
echo "  Base ref: $base_ref"

# Output the data in a format that can be easily parsed by the caller
cat <<EOF
MERGE_COMMIT_DATA
change_id=$local_merge_change_id
commit_id=$merge_commit
tree_hash=$tree_hash
base_ref=$base_ref
base_commit=$base_commit
EOF
