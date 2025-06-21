#!/usr/bin/env bash

set -euo pipefail
shopt -s nullglob

command -v jj >/dev/null 2>&1 || { echo "jj is required but not installed. Aborting."; exit 1; }

LOCAL_CI_PATH="$(unset GIT_DIR; cd "$(dirname "$(realpath "$0")")"; git rev-parse --show-toplevel)"

# Check command line arguments
if [[ $# -ne 2 ]]; then
    echo "Error: Expected exactly 2 arguments: pr_num jj_change_id" >&2
    echo "Usage: $0 <pr_num> <jj_change_id>" >&2
    exit 1
fi

readonly pr_num="$1"
readonly jj_change_id="$2"

# Validate pr_num is a positive integer
if ! [[ "$pr_num" =~ ^[1-9][0-9]*$ ]]; then
    echo "Error: pr_num must be a positive integer, got: $pr_num" >&2
    exit 1
fi

# Check required environment variable
if [[ -z "${LOCAL_CI_PATH:-}" ]]; then
    echo "Error: LOCAL_CI_PATH environment variable is not set" >&2
    exit 1
fi

# Check that compute_merge_description.py exists and is executable
readonly compute_script="$LOCAL_CI_PATH/sqlite/compute_merge_description.py"
if [[ ! -f "$compute_script" ]]; then
    echo "Error: compute_merge_description.py not found at: $compute_script" >&2
    exit 1
fi

if [[ ! -x "$compute_script" ]]; then
    echo "Error: compute_merge_description.py is not executable: $compute_script" >&2
    exit 1
fi

# Validate that the jj change ID exists
if ! jj log -r "$jj_change_id" --no-graph -T 'commit_id' >/dev/null 2>&1; then
    echo "Error: jj change ID does not exist: $jj_change_id" >&2
    exit 1
fi

echo "Recomputing merge description and performing sanity checks..."

# First, recompute description doing appropriate sanity checks
if ! description=$("$compute_script" -c "$jj_change_id" "$pr_num" 2>&1); then
    echo "Error: Failed to compute merge description" >&2
    echo "$description" >&2
    exit 1
fi

echo "Checking that computed description matches actual description..."

# Get the actual description of the jj change
if ! actual_description=$(jj log -r "$jj_change_id" --no-graph -T 'description' 2>/dev/null); then
    echo "Error: Failed to get description for jj change: $jj_change_id" >&2
    exit 1
fi

# Compare descriptions (normalize whitespace)
if [[ "$(echo "$description" | tr -s '[:space:]')" != "$(echo "$actual_description" | tr -s '[:space:]')" ]]; then
    echo "Error: Computed description does not match actual description" >&2
    echo "--- Computed description ---" >&2
    echo "$description" >&2
    echo "--- Actual description ---" >&2
    echo "$actual_description" >&2
    echo "--- End ---" >&2
    exit 1
fi

echo "Checking that change has exactly two parents..."

# Check that there are exactly two parents
if ! parents=$(jj log -r "$jj_change_id" --no-graph -T 'parents.map(|c| c.commit_id()).join(" ")' 2>/dev/null); then
    echo "Error: Failed to get parents for jj change: $jj_change_id" >&2
    exit 1
fi

# Count parents by splitting on whitespace
parent_array=($parents)
parent_count=${#parent_array[@]}

if [[ $parent_count -ne 2 ]]; then
    echo "Error: Change must have exactly 2 parents, found $parent_count" >&2
    echo "Parents: $parents" >&2
    exit 1
fi

echo "Change has exactly 2 parents:"
echo "Parent 1: ${parent_array[0]}"
echo "Parent 2: ${parent_array[1]}"

# Output jj log with the two parents
echo ""
echo "Showing merge and its parents:"
jj log -r "${parent_array[0]} | ${parent_array[1]} | $jj_change_id"
echo ""
echo "All checks passed."
echo ""
echo "WARNING: You are about to sign change $jj_change_id"
echo "This action cannot be undone. If the local CI bot is running,"
echo "it may push the commit to Github within seconds."
echo ""
read -p "Please type YES in all caps to confirm signing: " confirmation

if [[ "$confirmation" != "YES" ]]; then
    echo "Signing cancelled. You must type exactly 'YES' to confirm." >&2
    exit 1
fi

echo "Signing change..."

# Finally, if we make it this far, sign
if ! jj sign -r "$jj_change_id"; then
    echo "Error: Failed to sign change: $jj_change_id" >&2
    exit 1
fi

echo "Successfully signed change: $jj_change_id"
