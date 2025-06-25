#!/usr/bin/env bash

set -euo pipefail
shopt -s nullglob

command -v jj >/dev/null 2>&1 || { echo "jj is required but not installed. Aborting."; exit 1; }
command -v git >/dev/null 2>&1 || { echo "git is required but not installed. Aborting."; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo is required but not installed. Aborting."; exit 1; }
command -v gh >/dev/null 2>&1 || { echo "gh is required but not installed. Aborting."; exit 1; }

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

# First, recompute description doing appropriate sanity checks.
if ! description=$("$compute_script" -c "$jj_change_id" "$pr_num"); then
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
echo "All checks passed. Merge description:"
echo "-----"
jj log -r "$jj_change_id" -T description
echo "-----"
echo ""

# Check for version updates in Cargo.toml files
echo ""
echo "Checking for version updates in Cargo.toml files..."

# Get list of changed Cargo.toml files in this merge
changed_cargo_files=()
while read -r file; do
    if [[ "$file" == *"Cargo.toml" ]]; then
        changed_cargo_files+=("$file")
    fi
done < <(jj diff --to "$jj_change_id" --from "pull/$pr_num/base" --name-only 2>/dev/null)

if [[ ${#changed_cargo_files[@]} -eq 0 ]]; then
    echo "No Cargo.toml files were modified in this merge."
fi

# Function to extract package name and version from Cargo.toml
extract_package_info() {
    local cargo_file="$1"
    local commit_ref="$2"

    # Get the content of Cargo.toml at the specified commit
    local content
    if ! content=$(jj file show -r "$commit_ref" "$cargo_file" 2>/dev/null); then
        return 1
    fi

    # Extract name and version using basic parsing
    local name version
    name=$(echo "$content" | grep -E '^name\s*=' | head -1 | sed 's/^name\s*=\s*"\([^"]*\)".*/\1/')
    version=$(echo "$content" | grep -E '^version\s*=' | head -1 | sed 's/^version\s*=\s*"\([^"]*\)".*/\1/')

    if [[ -n "$name" && -n "$version" ]]; then
        echo "$name $version"
        return 0
    else
        return 1
    fi
}

# Check each Cargo.toml for version changes
packages_to_tag=()
for cargo_file in "${changed_cargo_files[@]}"; do
    echo ""
    echo "Checking $cargo_file for changes..."

    # Get package info from both parents
    base_info=""
    current_info=""

    if extract_package_info "$cargo_file" "pull/$pr_num/base" >/dev/null 2>&1; then
        base_info=$(extract_package_info "$cargo_file" "${parent_array[0]}")
    fi

    if extract_package_info "$cargo_file" "$jj_change_id" >/dev/null 2>&1; then
        current_info=$(extract_package_info "$cargo_file" "$jj_change_id")
    else
        echo "Warning: Could not extract package info from $cargo_file in merge commit"
        continue
    fi

    # Check if version changed from either parent
    current_name=$(echo "$current_info" | cut -d' ' -f1)
    current_version=$(echo "$current_info" | cut -d' ' -f2)

    version_changed=false

    if [[ -n "$base_info" ]]; then
        base_version=$(echo "$base_info" | cut -d' ' -f2)
        if [[ "$current_version" != "$base_version" ]]; then
            version_changed=true
            echo "Version changed from base: $base_version -> $current_version"
        fi
    fi

    if [[ "$version_changed" == true ]]; then
        echo "Package $current_name version updated to $current_version"
        packages_to_tag+=("$current_name $current_version")
    else
        echo "No version change detected for package $current_name"
    fi
done

if [[ ${#packages_to_tag[@]} -eq 0 ]]; then
    echo ""
    echo "No package version updates detected. No tags will be created."
    echo ""
else
    echo ""
    echo "Found ${#packages_to_tag[@]} package(s) with version updates:"
    printf '%s\n' "${packages_to_tag[@]}"
    echo ""
    echo "After signing, you will be given an opportunity to tag and publish"
    echo "the new version. (Typing YES to signing will not tag and publish;"
    echo "you will need to type YES a second time.)"
    echo ""
fi

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

# Get the commit ID for the signed change
if ! commit_id=$(jj log -r "$jj_change_id" --no-graph -T 'commit_id' 2>/dev/null); then
    echo "Error: Failed to get commit ID for jj change: $jj_change_id" >&2
    exit 1
fi

if [[ ${#packages_to_tag[@]} -eq 0 ]]; then
    exit 0
fi

# Create tags for each package
created_tags=()
for package_info in "${packages_to_tag[@]}"; do
    package_name=$(echo "$package_info" | cut -d' ' -f1)
    package_version=$(echo "$package_info" | cut -d' ' -f2)
    tag_name="${package_name}-${package_version}"

    echo ""
    echo "Creating signed tag: $tag_name"

    # Create signed tag
    if git tag -a -s "$tag_name" "$commit_id" -m "$tag_name"; then
        echo "Successfully created signed tag: $tag_name"
        echo "Tag points to commit: $commit_id"
        created_tags+=("$tag_name")
    else
        echo "Error: Failed to create tag: $tag_name" >&2
        exit 1
    fi
done

if [[ ${#created_tags[@]} -eq 0 ]]; then
    echo "No tags were created."
    exit 0
fi

echo ""
echo "Created ${#created_tags[@]} tag(s):"
printf '%s -> %s\n' "${created_tags[@]/#/}" "${created_tags[@]/#/$commit_id}"

# Ask for confirmation to publish
echo ""
echo "Do you want to publish these packages and push the tags?"
echo "This will:"
echo "- Checkout revision $commit_id in the current directory."
echo "- Push the new tags to the remote repository"
echo "- Run 'cargo publish -p <package>' for each package"
echo "- Post a GitHub comment 'Tagged and published.'"
echo ""
read -p "Type YES to proceed with publishing: " publish_confirmation

if [[ "$publish_confirmation" != "YES" ]]; then
    echo "Publishing cancelled. Tags have been created locally but not pushed."
    exit 0
fi

# In theory we should pull the remote name from the sqlite database, or take it
# on the command line, or something. In practice it is literally always `origin`.
remote_name="origin"
if ! git remote get-url "$remote_name" >/dev/null 2>&1; then
    echo "Error: Remote '$remote_name' not found" >&2
    exit 1
fi

#
if ! git checkout "$commit_id" >/dev/null 2>&1; then
    echo "Error: Failed to checkout $commit_id" >&2
    exit 1
fi

# Publish packages
echo ""
echo "Publishing packages..."
for package_info in "${packages_to_tag[@]}"; do
    package_name=$(echo "$package_info" | cut -d' ' -f1)
    echo "Publishing package: $package_name"
    if ! cargo publish -p "$package_name"; then
        echo "Error: Failed to publish package: $package_name" >&2
        exit 1
    fi
done

# Push tags
echo ""
echo "Pushing tags to remote '$remote_name'..."
for tag_name in "${created_tags[@]}"; do
    echo "Pushing tag: $tag_name"
    if ! git push "$remote_name" "$tag_name"; then
        echo "Error: Failed to push tag: $tag_name" >&2
        exit 1
    fi
done

# Post GitHub comment
echo ""
echo "Posting GitHub comment..."
if ! gh pr comment "$pr_num" --body "Tagged and published."; then
    echo "Error: Failed to post GitHub comment" >&2
    exit 1
fi

echo ""
echo "Successfully completed all publishing tasks:"
echo "- Created and pushed ${#created_tags[@]} tag(s)"
echo "- Published ${#packages_to_tag[@]} package(s)"
echo "- Posted GitHub comment"
