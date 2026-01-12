#!/usr/bin/env bash

set -euo pipefail
shopt -s nullglob

command -v gh >/dev/null 2>&1 || { echo "gh is required but not installed. Aborting." >&2; exit 1; }
command -v git >/dev/null 2>&1 || { echo "git is required but not installed. Aborting." >&2; exit 1; }
command -v jj >/dev/null 2>&1 || { echo "jj is required but not installed. Aborting." >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "jq is required but not installed. Aborting." >&2; exit 1; }
command -v nix >/dev/null 2>&1 || { echo "nix is required but not installed. Aborting." >&2; exit 1; }
command -v task >/dev/null 2>&1 || { echo "task is required but not installed. Aborting." >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { echo "sha256sum is required but not installed. Aborting." >&2; exit 1; }
if [[ ! "$(task --version)" =~ ^3 ]]; then
    echo "Need taskwarrior version 3. Aborting" >&2
    exit 2
fi  

####
# Global setup
#
# You may want to edit these variables.
####
export TASKRC="$HOME/.config/local-ci.taskrc"
export TASKDATA="$HOME/.local-ci.task"
NIX_PIN_PATH="$HOME/code/NixOS/nixpkgs/local-ci-pin/"

NIXPKGS_COMMIT_ID=$(cd "$NIX_PIN_PATH/" && git rev-parse HEAD)
LOCAL_CI_PATH="$(unset GIT_DIR; cd "$(dirname "$(realpath "$0")")"; git rev-parse --show-toplevel)"
LOCAL_CI_WORKTREE="../local-ci-worktree"
LOCAL_CI_COMMIT_ID="$(unset GIT_DIR; cd "$(dirname "$(realpath "$0")")"; git rev-parse HEAD)"
LOCAL_CI_DIFF="$(unset GIT_DIR; cd "$(dirname "$(realpath "$0")")"; git diff)"

export NIX_PATH=nixpkgs=$NIX_PIN_PATH

####
# Startup sanity checks
###

HOSTNAME=$(cat /etc/hostname || echo "$HOST" || hostname)
if [ -z "$HOSTNAME" ]; then
    echo "Your hostname appears to be unset. Please set your hostname to something unique." >&2
    exit 2
fi

# We reject a hostname of 'localhost' on the theory that it will cause
# problems for people syncing multiple boxes. We may want to remove this
# check if it annoys people since the use of multiple boxes is likely to
# be unusual.
if [ "$HOSTNAME" == "localhost" ]; then
    echo "Your hostname appears to be 'localhost'. Please set your hostname to something unique." >&2
    exit 2
fi

# Check whether we are logged into Github
GITHUB_STATUS=$(gh auth status --json hosts --jq '.hosts."github.com".[0].active')
if [ "$GITHUB_STATUS" != "true" ]; then
    echo "gh appears not to be logged into Github." >&2
    echo "We make a lot of queuries and need to be authenticated." >&2
    echo "Run 'gh auth' to authenticate." >&2
    echo >&2
    echo "Output of 'gh auth status':" >&2
    gh auth status --json hosts >&2
    echo >&2
    echo "Failing." >&2
    exit 2
fi

####
# Initialize configuration and source CLI commands
####
pushd "$LOCAL_CI_PATH"/taskwarrior > /dev/null
source ./init.sh
source ./queue.sh
source ./run.sh
popd > /dev/null

###
# Utility functions
###

# Sets the REPO_ROOT and PROJECT environment variables.
locate_repo() {
    # Use 'gh' to determine project name.
    # FIXME we should have some fallbacks here.
    PROJECT=$(gh repo view --json 'owner,name' --jq '.owner.login + "." + .name')
    REPO_ROOT=$(git rev-parse --show-toplevel)

    export REPO_ROOT
    export PROJECT
}

tw_unique_uuid() {
    if [[ $# -lt 2 ]]; then
        echo "usage: tw_uuid <search filter...>" >&2
        return 2
    fi

    shift
    local uuids
    local count
    uuids=$(task $@ export | jq -r '.[].uuid // empty')
    count=$(echo "$uuids" | wc -w)
    if [ "$count" -gt 1 ]; then
        echo "Error: Multiple UUIDs found for filter $@" >&2
        echo "UUIDs: $uuids" >&2
        exit 1
    else
        echo "$uuids" # either empty or a single UUID
    fi
}

tw_upsert() {
    if [[ $# -lt 2 ]]; then
        echo "usage: tw_upsert <search filter...> -- <modify filter...>" >&2
        return 2
    fi

    # Split args at the literal "--"
    local -a filter modify
    local saw_sep=0
    for a in "$@"; do
        if [[ $saw_sep -eq 0 && $a == "--" ]]; then
            saw_sep=1
            continue
        fi
        if [[ $saw_sep -eq 0 ]]; then
            filter+=("$a")
        else
            modify+=("$a")
        fi
    done
    if [[ $saw_sep -eq 0 ]]; then
        echo "tw_upsert: missing '--' separator" >&2
        return 2
    fi

    local uuid
    uuid="$(tw_unique_uuid "${filter[@]}")"
    if [ -z "$uuid" ]; then
        # Create new task and capture its UUID.
        uuid="$(task rc.verbose=new-uuid add "${filter[@]}" "${modify[@]}" rc.confirmation=off | sed -n 's/.*Created task \([0-9a-f-]\{36\}\).*/\1/p')"
        if [[ -z $uuid ]]; then
            echo "tw_upsert: failed to extract uuid from add output" >&2
            return 1
        fi
    else
        task rc.verbose=nothing "$uuid" modify "${modify[@]}"
    fi

    echo "$uuid"
}

check_for_pushable_merges() {
    local commit_uuid="$1"
    
    # Check if this is a merge commit that just succeeded
    local commit_data=$(task "$commit_uuid" export | jq -r '.[0]')
    local is_merge=$(echo "$commit_data" | jq -r '.tags // [] | contains(["MERGE_COMMIT"])')
    local ci_status=$(echo "$commit_data" | jq -r '.ci_status // "unstarted"')
    
    if [ "$is_merge" = "true" ] && [ "$ci_status" = "success" ]; then
        local commit_id=$(echo "$commit_data" | jq -r '.commit_id')
        local tree_hash=$(git rev-parse "$commit_id^{tree}" 2>/dev/null || echo "")
        
        if [ -n "$tree_hash" ]; then
            # Find PRs with matching tree hash and update their merge_status
            local pr_uuids=$(task "tree_hash:$tree_hash" "pr_number.any:" export | jq -r '.[].uuid')
            for pr_uuid in $pr_uuids; do
                if [ -n "$pr_uuid" ]; then
                    local pr_number=$(task "$pr_uuid" export | jq -r '.[0].pr_number')
                    echo "Updating merge_status to needsig for PR #$pr_number (tree hash matches)"
                    task "$pr_uuid" modify "merge_status:needsig"
                fi
            done
        fi
    fi
}

post_github_approval_if_ready() {
    local pr_uuid="$1"
    
    local pr_data=$(task "$pr_uuid" export | jq -r '.[0]')
    local pr_number=$(echo "$pr_data" | jq -r '.pr_number')
    local pr_review_status=$(echo "$pr_data" | jq -r '.review_status // "unreviewed"')
    local repo_root=$(echo "$pr_data" | jq -r '.repo_root')
    
    # Only proceed if PR is approved
    if [ "$pr_review_status" != "approved" ]; then
        return
    fi
    
    # Check if all commits are approved and CI successful
    local all_commits_approved_and_ci=true
    local commit_uuids=$(task "depends:$pr_uuid" export | jq -r '.[] | select(.commit_id) | .uuid')
    
    for commit_uuid_check in $commit_uuids; do
        if [ -n "$commit_uuid_check" ]; then
            local commit_review_status=$(task "$commit_uuid_check" export | jq -r '.[0].review_status // "unreviewed"')
            local commit_ci_status=$(task "$commit_uuid_check" export | jq -r '.[0].ci_status // "unstarted"')
            if [ "$commit_review_status" != "approved" ] || [ "$commit_ci_status" != "success" ]; then
                all_commits_approved_and_ci=false
                break
            fi
        fi
    done
    
    # Only post approval if all conditions are met
    if [ "$all_commits_approved_and_ci" != "true" ]; then
        echo "PR #$pr_number is approved but not all commits are approved and CI successful yet"
        return
    fi
    
    # Get all commits for this PR
    local all_commits_successful=true
    local commit_uuids=$(task "depends:$pr_uuid" export | jq -r '.[] | select(.commit_id) | .uuid')
    
    for commit_uuid_check in $commit_uuids; do
        if [ -n "$commit_uuid_check" ]; then
            local commit_ci_status=$(task "$commit_uuid_check" export | jq -r '.[0].ci_status // "unstarted"')
            if [ "$commit_ci_status" != "success" ]; then
                all_commits_successful=false
                break
            fi
        fi
    done
    
    # If all commits successful and PR is approved, post approval on GitHub
    if [ "$all_commits_successful" = "true" ]; then
        echo "All commits in PR #$pr_number are successful and PR is approved. Posting GitHub approval..."
        
        # Get PR review notes
        local pr_review_notes=$(task "$pr_uuid" export | jq -r '.[0].review_notes // ""')
        pushd "$repo_root" > /dev/null
        if gh pr review "$pr_number" -a -b "$pr_review_notes"; then
            echo "Successfully posted approval for PR #$pr_number"
        else
            echo "Failed to post approval for PR #$pr_number - posting comment instead"
            gh pr review "$pr_number" -c -b "$pr_review_notes"
        fi
        popd > /dev/null
    fi
}

####
# Main logic
####

if [ ! -e "$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE" ]; then
    echo "Warning: creating worktree at $LOCAL_CI_PATH/$LOCAL_CI_WORKTREE."
    echo "Any actions taken in this worktree are likely to be overwritten."
    pushd "$LOCAL_CI_PATH"
    git worktree add "$LOCAL_CI_WORKTREE"
    popd
fi

ARG_COMMAND=
ARG_COMMAND_ARGS=()
ARG_ALLOW_DIRTY_LOCAL_CI="no"

parse_arguments() {
    local args=("$@")
    for ((i = 0; i < ${#args[@]}; i++)); do
        if [[ "${args[i]}" == "--allow-dirty-local-ci" ]]; then
            ARG_ALLOW_DIRTY_LOCAL_CI="yes"
        else
            if [ "$ARG_COMMAND" == "" ]; then
                ARG_COMMAND=${args[i]}
            else
                # Any unrecognized flags after the command are given to the command as arguments
                ARG_COMMAND_ARGS+=("${args[i]}")
            fi
        fi
    done
}
parse_arguments "$@"

if [[ "$ARG_ALLOW_DIRTY_LOCAL_CI" != "yes" ]] && [[ ! -z "$LOCAL_CI_DIFF" ]]; then
    echo "local CI directory appears to be dirty and --allow-dirty-local-ci was not passed"
    exit 1
fi

case "$ARG_COMMAND" in
    queue-commit)
        queue_commit "${ARG_COMMAND_ARGS[@]}"
        ;;
    list)
        task list
        ;;
    pr)
        # First, sanity-check the PR number
        pr_num="${ARG_COMMAND_ARGS[0]:-}"
        pr_subcommand="${ARG_COMMAND_ARGS[1]:-}"
        
        case $pr_num in
            '')
                echo "PR number is required by pr command"
                exit 1
                ;;
            *[!0-9]*)
                echo "PR number must be a number, not $pr_num"
                exit 1
                ;;
        esac
        
        # Handle subcommands
        case $pr_subcommand in
            task-info)
                locate_repo
                task "project:local-ci.$PROJECT" "pr_number:$pr_num" commit_id: info
                exit 0
                ;;
            task-edit)
                locate_repo
                UUID=$(tw_unique_uuid "project:local-ci.$PROJECT" "pr_number:$pr_num" commit_id:)
                task "$UUID" edit
                exit 0
                ;;
            ''|info)
                # Default behavior - show PR info
                ;;
            review)
                # PR review workflow
                ;;
            *)
                echo "Unknown pr subcommand: $pr_subcommand"
                echo "Available pr subcommands: info (default), task-info, task-edit, review"
                exit 1
                ;;
        esac
        
        # Figure out where we are and query Github for the PR info.
        locate_repo
        JSON_DATA=$(gh pr view "$pr_num" --json commits,title,author,potentialMergeCommit,baseRefName | jq -c)
        
        # Extract PR data
        PR_TITLE=$(echo "$JSON_DATA" | jq -r .title)
        PR_AUTHOR=$(echo "$JSON_DATA" | jq -r .author.login)
        PR_COMMITS=$(echo "$JSON_DATA" | jq -r '.commits[].oid')
        PR_MERGE_COMMIT=$(echo "$JSON_DATA" | jq -r '.potentialMergeCommit.oid // empty')
        PR_BASE_REF=$(echo "$JSON_DATA" | jq -r '.baseRefName // "master"')
        
        # Handle merge commit data - check if we need to create/recreate merge commit
        MERGE_TREE_HASH=""
        MERGE_JJ_CHANGE_ID=""
        MERGE_COMMIT_ID=""
        MERGE_BASE_COMMIT=""
        
        # Get current PR data to check if merge commit exists and is still valid
        PR_DATA=$(task "project:local-ci.$PROJECT" "pr_number:$pr_num" export 2>/dev/null | jq -r '.[0] // {}')
        CURRENT_BASE_COMMIT=$(echo "$PR_DATA" | jq -r '.base_commit // ""')
        CURRENT_JJ_CHANGE_ID=$(echo "$PR_DATA" | jq -r '.jj_change_id // ""')
        
        # Get the current base commit from GitHub
        FRESH_BASE_COMMIT=$(git rev-parse "pull/$pr_num/base" 2>/dev/null || echo "")
        
        # Only create merge commit if it doesn't exist or base commit has changed
        if [ -z "$CURRENT_JJ_CHANGE_ID" ] || [ "$CURRENT_BASE_COMMIT" != "$FRESH_BASE_COMMIT" ]; then
            if [ -n "$CURRENT_JJ_CHANGE_ID" ] && [ "$CURRENT_BASE_COMMIT" != "$FRESH_BASE_COMMIT" ]; then
                echo "Base commit changed from $CURRENT_BASE_COMMIT to $FRESH_BASE_COMMIT, recreating merge commit..."
            else
                echo "Creating synthetic merge commit for PR #$pr_num..."
            fi
            
            MERGE_OUTPUT=$(mktemp)
            if "$LOCAL_CI_PATH/taskwarrior/create-merge-commit.sh" "$pr_num" 3> "$MERGE_OUTPUT"; then
                # Parse the output to get merge commit data
                if grep -q "MERGE_COMMIT_DATA" "$MERGE_OUTPUT"; then
                    eval $(grep -A5 "MERGE_COMMIT_DATA" "$MERGE_OUTPUT" | tail -5)
                    MERGE_JJ_CHANGE_ID="$change_id"
                    MERGE_COMMIT_ID="$commit_id"
                    MERGE_TREE_HASH="$tree_hash"
                    MERGE_BASE_COMMIT="$base_commit"
                    echo "Successfully created merge commit:"
                    echo "  Change ID: $MERGE_JJ_CHANGE_ID"
                    echo "  Commit ID: $MERGE_COMMIT_ID"
                    echo "  Tree hash: $MERGE_TREE_HASH"
                    echo "  Base commit: $MERGE_BASE_COMMIT"
                else
                    echo "Warning: Could not parse merge commit data from output"
                    cat "$MERGE_OUTPUT"
                fi
                rm "$MERGE_OUTPUT"
            else
                echo "Warning: failed to create synthetic merge commit:"
                cat "$MERGE_OUTPUT"
                rm "$MERGE_OUTPUT" || true
            fi
        else
            echo "Using existing merge commit (base commit unchanged)"
            MERGE_JJ_CHANGE_ID="$CURRENT_JJ_CHANGE_ID"
            MERGE_BASE_COMMIT="$CURRENT_BASE_COMMIT"
            
            # Get other merge commit details from JJ
            if command -v jj >/dev/null 2>&1; then
                MERGE_COMMIT_ID=$(jj log --no-graph -r "$MERGE_JJ_CHANGE_ID" -T commit_id 2>/dev/null || echo "")
                if [ -n "$MERGE_COMMIT_ID" ]; then
                    MERGE_TREE_HASH=$(git rev-parse "$MERGE_COMMIT_ID^{tree}" 2>/dev/null || echo "")
                fi
            fi
        fi

        # Create/modify PR task.
        PR_FILTER=(
            "project:local-ci.$PROJECT"
            "pr_number:$pr_num"
            commit_id:
        )
        PR_UPSERT=(
            "repo_root:$REPO_ROOT"
            "pr_title:$PR_TITLE"
            "pr_author:$PR_AUTHOR"
            "description:PR #$pr_num: $PR_TITLE"
        )
        
        # Add merge commit data if available
        if [ -n "$MERGE_BASE_COMMIT" ]; then
            PR_UPSERT+=("base_commit:$MERGE_BASE_COMMIT")
        fi
        if [ -n "$MERGE_JJ_CHANGE_ID" ]; then
            PR_UPSERT+=("jj_change_id:$MERGE_JJ_CHANGE_ID")
        fi
        
        PR_UUID=$(tw_upsert "${PR_FILTER[@]}" -- "${PR_UPSERT[@]}")

        # Get current dependencies to check for changes
        CURRENT_DEPS=$(task "$PR_UUID" export | jq -r '.[0].depends // [] | .[]')
        
        # Now handle individual commit tasks
        COMMIT_UUIDS=()
        for commit_id in $PR_COMMITS; do
            if [ -n "$commit_id" ]; then
                COMMIT_FILTER=(
                    "project:local-ci.$PROJECT"
                    "commit_id:$commit_id"
                )
                COMMIT_UPSERT=(
                    "repo_root:$REPO_ROOT"
                    "description:Commit $commit_id"
                )
                COMMIT_UUID=$(tw_upsert "${COMMIT_FILTER[@]}" -- "${COMMIT_UPSERT[@]}")
                COMMIT_UUIDS+=("$COMMIT_UUID")
                task "$PR_UUID" modify "depends:$COMMIT_UUID"
            fi
        done

        if [ -n "$COMMIT_UUID" ]; then
            task "$COMMIT_UUID" modify +TIP_COMMIT
        fi
        
        # Handle merge commit - use our synthetic merge commit
        MERGE_COMMIT_UUID=""
        if [ -n "$MERGE_COMMIT_ID" ]; then
            # Create merge commit task
            MERGE_COMMIT_FILTER=(
                "project:local-ci.$PROJECT"
                "commit_id:$MERGE_COMMIT_ID"
            )
            MERGE_COMMIT_UPSERT=(
                "repo_root:$REPO_ROOT"
                "description:Merge commit $MERGE_COMMIT_ID for PR #$pr_num"
            )
            MERGE_COMMIT_UUID=$(tw_upsert "${MERGE_COMMIT_FILTER[@]}" -- "${MERGE_COMMIT_UPSERT[@]}")
            task "$MERGE_COMMIT_UUID" modify +MERGE_COMMIT
            task "$PR_UUID" modify "depends:$MERGE_COMMIT_UUID"
            COMMIT_UUIDS+=("$MERGE_COMMIT_UUID")
        fi
        
        # Remove old dependencies that are no longer part of this PR
        NEW_DEPS=("${COMMIT_UUIDS[@]}")
        for old_dep in $CURRENT_DEPS; do
            if [ -n "$old_dep" ]; then
                # Check if this dependency is still needed
                found=false
                for new_dep in "${NEW_DEPS[@]}"; do
                    if [ "$old_dep" = "$new_dep" ]; then
                        found=true
                        break
                    fi
                done
                
                if [ "$found" = "false" ]; then
                    # This dependency is no longer needed, remove it
                    task "$PR_UUID" modify "depends:-$old_dep"
                    echo "Removed old dependency: $old_dep"
                fi
            fi
        done
        
        echo "Finished processing PR $pr_num. Task UUID $PR_UUID"
        echo
        
        # Display PR information
        echo "=== PR #$pr_num: $PR_TITLE ==="
        echo "Author: $PR_AUTHOR"
        
        # Get PR review status
        PR_REVIEW_STATUS=$(task "$PR_UUID" export | jq -r '.[0].review_status // "unreviewed"')
        echo "PR Review Status: $PR_REVIEW_STATUS"
        PR_MERGE_STATUS=$(task "$PR_UUID" export | jq -r '.[0].merge_status // "unreviewed"')
        echo "PR Merge Status: $PR_MERGE_STATUS"
        echo
        
        # Display commit information
        echo "=== Commits ==="
        
        HAS_NACKED_COMMIT=false
        HAS_APPROVED_COMMIT=false
        HAS_FAILED_CI=false
        
        for commit_uuid in "${COMMIT_UUIDS[@]}"; do
            if [ -n "$commit_uuid" ]; then
                COMMIT_DATA=$(task "$commit_uuid" export | jq -r '.[0]')
                COMMIT_ID=$(echo "$COMMIT_DATA" | jq -r '.commit_id // ""')
                COMMIT_REVIEW_STATUS=$(echo "$COMMIT_DATA" | jq -r '.review_status // "unreviewed"')
                COMMIT_CI_STATUS=$(echo "$COMMIT_DATA" | jq -r '.ci_status // "unstarted"')
                IS_TIP=$(echo "$COMMIT_DATA" | jq -r '.tags // [] | contains(["TIP_COMMIT"])')
                IS_MERGE=$(echo "$COMMIT_DATA" | jq -r '.tags // [] | contains(["MERGE_COMMIT"])')
                
                echo -n "  $COMMIT_ID (review: $COMMIT_REVIEW_STATUS"
                
                if [ "$COMMIT_REVIEW_STATUS" = "approved" ]; then
                    echo -n ", ci: $COMMIT_CI_STATUS"
                    HAS_APPROVED_COMMIT=true
                fi
                
                if [ "$COMMIT_REVIEW_STATUS" = "nacked" ]; then
                    HAS_NACKED_COMMIT=true
                fi
                
                if [ "$COMMIT_CI_STATUS" = "failed" ]; then
                    HAS_FAILED_CI=true
                fi
                
                if [ "$IS_TIP" = "true" ]; then
                    echo -n ", TIP"
                fi
                
                if [ "$IS_MERGE" = "true" ]; then
                    echo -n ", MERGE"
                fi
                
                echo ")"
            fi
        done
        
        # Display merge commit status
        echo
        echo "=== Merge Commit ==="
        if [ -n "$MERGE_COMMIT_ID" ]; then
            MERGE_DATA=$(task "$MERGE_COMMIT_UUID" export | jq -r '.[0]')
            MERGE_REVIEW_STATUS=$(echo "$MERGE_DATA" | jq -r '.review_status // "unreviewed"')
            MERGE_CI_STATUS=$(echo "$MERGE_DATA" | jq -r '.ci_status // "unstarted"')
            echo -n "  Merge commit: $MERGE_COMMIT_ID (review: $MERGE_REVIEW_STATUS"
            if [ "$MERGE_REVIEW_STATUS" = "approved" ]; then
                echo -n ", ci: $MERGE_CI_STATUS"
            fi
            echo ")"

            if [ -n "$MERGE_TREE_HASH" ]; then
                echo "  Tree hash: $MERGE_TREE_HASH"
            fi
            if [ -n "$MERGE_BASE_COMMIT" ]; then
                echo "  Base commit: $MERGE_BASE_COMMIT"
            fi
            if [ -n "$MERGE_JJ_CHANGE_ID" ]; then
                echo "  JJ change ID: $MERGE_JJ_CHANGE_ID"
            fi
            echo "  (Review the merge commit like any other commit to trigger CI)"
        else
            echo "  NO MERGE COMMIT CREATED"
        fi
        
        echo
        
        # Handle review subcommand
        if [ "$pr_subcommand" = "review" ]; then
            # PR review workflow
            TIP_COMMIT=
            while true; do
                echo
                echo "=== Reviewing PR #$pr_num: $PR_TITLE ==="
                echo "Author: $PR_AUTHOR"
                echo "Current PR Review Status: $PR_REVIEW_STATUS"
                echo
                
                # Show commit status summary
                echo "=== Commit Status Summary ==="
                ALL_COMMITS_APPROVED=true
                ALL_COMMITS_CI_SUCCESS=true
                
                for commit_uuid in "${COMMIT_UUIDS[@]}"; do
                    if [ -n "$commit_uuid" ]; then
                        COMMIT_DATA=$(task "$commit_uuid" export | jq -r '.[0]')
                        COMMIT_ID=$(echo "$COMMIT_DATA" | jq -r '.commit_id // ""')
                        COMMIT_REVIEW_STATUS=$(echo "$COMMIT_DATA" | jq -r '.review_status // "unreviewed"')
                        COMMIT_CI_STATUS=$(echo "$COMMIT_DATA" | jq -r '.ci_status // "unstarted"')
                        IS_TIP=$(echo "$COMMIT_DATA" | jq -r '.tags // [] | contains(["TIP_COMMIT"])')
                        
                        echo -n "  $COMMIT_ID: review=$COMMIT_REVIEW_STATUS, ci=$COMMIT_CI_STATUS"
                        if [ "$IS_TIP" = "true" ]; then
                            TIP_COMMIT=$COMMIT_ID
                            echo -n " (TIP)"
                        fi
                        echo
                        
                        if [ "$COMMIT_REVIEW_STATUS" != "approved" ]; then
                            ALL_COMMITS_APPROVED=false
                        fi
                        if [ "$COMMIT_CI_STATUS" != "success" ]; then
                            ALL_COMMITS_CI_SUCCESS=false
                        fi
                    fi
                done
                
                echo
                echo "All commits approved: $ALL_COMMITS_APPROVED"
                echo "All commits CI success: $ALL_COMMITS_CI_SUCCESS"
                echo
                
                echo "What would you like to do?"
                echo "1) Approve PR"
                echo "2) NACK PR"
                echo "3) Request changes"
                echo "4) View total diff"
                echo "5) Cancel"
                read -p "Choice (1-5): " choice
                
                case "$choice" in
                    1)
                        NEW_STATUS="approved"
                        echo "Approving PR..."
                        ;;
                    2) NEW_STATUS="nacked" ;;
                    3) NEW_STATUS="needschange" ;;
                    4)
                        echo "--- Total diff for PR #$pr_num ---"
                        pushd "$REPO_ROOT" > /dev/null
                        gh pr diff "$pr_num"
                        popd > /dev/null
                        continue
                        ;;
                    5)
                        echo "Review cancelled."
                        break
                        ;;
                    *)
                        echo "Invalid choice. Please select 1-5."
                        continue
                        ;;
                esac

                if [ -n "$NEW_STATUS" ]; then
                    # Get tip commit for the approval message
                    if [ -z "$TIP_COMMIT" ]; then
                        echo "Warning: PR appears to have no tip commit set; please manually fix this. Cannot review." >&2
                        break
                    fi

                    # Open editor for review notes
                    TEMP_FILE=$(mktemp)
                    # Populate temp file with template
                    echo "# Enter your PR review here. Updated PR #$pr_num review status: $NEW_STATUS" >> "$TEMP_FILE"
                    if [ "$NEW_STATUS" = "approved" ]; then
                        echo "# This will be posted as a Github approval as soon as all CI runs have passed" >> "$TEMP_FILE"
                        echo "# and all commits are approved." >> "$TEMP_FILE"
                        echo "ACK $TIP_COMMIT; successfully ran local tests" > "$TEMP_FILE"
                    fi

                    echo "# Commit Review Information:" >> "$TEMP_FILE"
                    # Add commit review information
                    for commit_uuid_check in "${COMMIT_UUIDS[@]}"; do
                        if [ -n "$commit_uuid_check" ]; then
                            commit_data=$(task "$commit_uuid_check" export | jq -r '.[0]')
                            commit_id=$(echo "$commit_data" | jq -r '.commit_id')
                            commit_review_notes=$(echo "$commit_data" | jq -r '.review_notes // ""')
                            is_tip=$(echo "$commit_data" | jq -r '.tags // [] | contains(["TIP_COMMIT"])')
                
                            echo -n "# $commit_id" >> "$TEMP_FILE"
                            if [ "$is_tip" = "true" ]; then
                                echo -n " (TIP)" >> "$TEMP_FILE"
                            fi
                            echo ":" >> "$TEMP_FILE"
                            if [ -n "$commit_review_notes" ]; then
                                echo "#   Review: $commit_review_notes" >> "$TEMP_FILE"
                            else
                                echo "#   Review: (none)" >> "$TEMP_FILE"
                            fi
                        fi
                    done
                    echo "# Edit the approval message above. Lines starting with # will be removed." >> "$TEMP_FILE"
                    EDITOR_CMD="${EDITOR:-vim}"
                
                    echo "Opening $EDITOR_CMD for review notes..."
                    if "$EDITOR_CMD" "$TEMP_FILE"; then
                        # Read review notes from temp file and remove comment lines
                        REVIEW_NOTES=$(grep -v '^#' "$TEMP_FILE" | sed '/^$/d')
                        rm "$TEMP_FILE"
                    
                        # Update task with new status and notes
                        task "$PR_UUID" modify "review_status:$NEW_STATUS" "review_notes:$REVIEW_NOTES"
                    
                        echo "PR #$pr_num review status updated to: $NEW_STATUS"
                        if [ -n "$REVIEW_NOTES" ]; then
                            echo "Review notes saved."
                        fi
                    
                        # Check if we should post GitHub approval
                        post_github_approval_if_ready "$PR_UUID"
                        break
                    else
                        # Editor failed (e.g. user typed :cq in vim)
                        rm "$TEMP_FILE"
                        echo "Editor exited with error. Review cancelled."
                        continue
                    fi
                fi
            done
        else
            # Provide suggestions based on review status (info subcommand)
            if [ "$HAS_NACKED_COMMIT" = "true" ] && [ "$PR_REVIEW_STATUS" != "nacked" ]; then
                echo "⚠️  Note: Some commits are nacked but the PR is not nacked."
                echo "   Consider nacking the PR with: $0 pr $pr_num review"
                echo
            fi
            
            if [ "$HAS_FAILED_CI" = "true" ]; then
                echo "⚠️  Note: Some commits have failed CI status."
                echo "   To restart CI for a commit, use: $0 commit <commit_id> reset-ci"
                echo
            fi
        fi
        ;;
    run)
        # Reset any "started" jobs to "unstarted" (previous instance likely crashed)
        echo "Resetting any 'started' jobs to 'unstarted'..."
        task "ci_status:started" modify "ci_status:unstarted" || echo "(none updated)"
        
        run_ci_loop
        ;;
    nack)
        # Handle PR nack command
        pr_num="${ARG_COMMAND_ARGS[0]:-}"
        case $pr_num in
            '')
                echo "PR number is required by nack command"
                exit 1
                ;;
            *[!0-9]*)
                echo "PR number must be a number, not $pr_num"
                exit 1
                ;;
        esac
        
        locate_repo
        PR_FILTER=(
            "project:local-ci.$PROJECT"
            "pr_number:$pr_num"
            commit_id:
        )
        PR_UUID=$(tw_unique_uuid "${PR_FILTER[@]}")
        if [ -z "$PR_UUID" ]; then
            echo "PR #$pr_num not found in task database"
            exit 1
        fi
        
        task "$PR_UUID" modify "review_status:nacked"
        echo "PR #$pr_num marked as nacked"
        ;;
    commit)
        # Handle commit commands
        if [ "${#ARG_COMMAND_ARGS[@]}" -lt 2 ]; then
            echo "Usage: $0 commit <commit_id> {review|reset-ci}"
            exit 1
        fi
        
        subcommand="${ARG_COMMAND_ARGS[1]}"
        if [ "$subcommand" != "review" ] && [ "$subcommand" != "reset-ci" ]; then
            echo "Usage: $0 commit <commit_id> {review|reset-ci}"
            exit 1
        fi
        
        commit_id="${ARG_COMMAND_ARGS[0]:-}"
        if [ -z "$commit_id" ]; then
            echo "Commit ID is required"
            exit 1
        fi
        
        locate_repo
        
        # Resolve short commit ID to full commit ID
        FULL_COMMIT_ID=$(git rev-parse "$commit_id" 2>/dev/null || echo "")
        if [ -z "$FULL_COMMIT_ID" ]; then
            echo "Invalid commit ID: $commit_id"
            exit 1
        fi
        
        # Create/find commit task
        COMMIT_FILTER=(
            "project:local-ci.$PROJECT"
            "commit_id:$FULL_COMMIT_ID"
        )
        COMMIT_UPSERT=(
            "repo_root:$REPO_ROOT"
            "description:Commit $FULL_COMMIT_ID"
        )
        COMMIT_UUID=$(tw_upsert "${COMMIT_FILTER[@]}" -- "${COMMIT_UPSERT[@]}")
        
        if [ "$subcommand" = "reset-ci" ]; then
            # Reset CI status to unstarted
            task "$COMMIT_UUID" modify "ci_status:unstarted"
            echo "CI status for commit $FULL_COMMIT_ID reset to unstarted"
            exit 0
        fi
        
        # Get current review status (for review command)
        CURRENT_STATUS=$(task "$COMMIT_UUID" export | jq -r '.[0].review_status // "unreviewed"')
        
        while true; do
            echo
            echo "=== Reviewing commit $FULL_COMMIT_ID ==="
            echo "Current review status: $CURRENT_STATUS"
            echo
            
            # Show git diff
            echo "--- Git diff ---"
            git show "$FULL_COMMIT_ID"
            echo
            
            # Show PRs containing this commit
            echo "--- PRs containing this commit ---"
            PR_TASKS=$(task "project:local-ci.$PROJECT" "depends:$COMMIT_UUID" export 2>/dev/null || echo "[]")
            if [ "$PR_TASKS" != "[]" ]; then
                echo "$PR_TASKS" | jq -r '.[] | "  PR #" + (.pr_number | tostring) + ": " + .pr_title + " (by " + .pr_author + ")"'
                
                # Check if this is the tip commit for any PRs
                IS_TIP=$(task "$COMMIT_UUID" export | jq -r '.[0].tags // [] | contains(["TIP_COMMIT"])')
                if [ "$IS_TIP" = "true" ]; then
                    echo "  ⚠️  This is a tip commit for at least one of the above PR(s)"
                fi
                echo
                echo "  Note: Remember to review the PR(s) separately from individual commits."
            else
                echo "  No PRs found containing this commit."
            fi
            echo
            
            # Prompt for action
            echo "What would you like to do?"
            echo "1) Approve"
            echo "2) NACK"
            echo "3) Needs Change"
            echo "4) Erase review (mark unreviewed)"
            echo "5) Re-view diff"
            echo "6) Cancel"
            read -p "Choice (1-6): " choice
            
            case "$choice" in
                1) NEW_STATUS="approved" ;;
                2) NEW_STATUS="nacked" ;;
                3) NEW_STATUS="needschange" ;;
                4) NEW_STATUS="unreviewed" ;;
                5)
                    # Continue loop to re-show diff
                    continue
                    ;;
                6)
                    echo "Review cancelled."
                    break
                    ;;
                *)
                    echo "Invalid choice. Please select 1-6."
                    continue
                    ;;
            esac
                    
            # Open editor for review notes
            TEMP_FILE=$(mktemp)
            EDITOR_CMD="${EDITOR:-vim}"
            
            # Populate temp file with template
            echo "# Enter your review here. Updated commit $FULL_COMMIT_ID review status: $NEW_STATUS" > "$TEMP_FILE"
            
            echo "Opening $EDITOR_CMD for review notes..."
            if "$EDITOR_CMD" "$TEMP_FILE"; then
                # Read review notes from temp file and remove comment lines
                REVIEW_NOTES=$(grep -v '^#' "$TEMP_FILE" | sed '/^$/d')
                rm "$TEMP_FILE"
                
                # Update task with new status and notes
                task "$COMMIT_UUID" modify "review_status:$NEW_STATUS" "review_notes:$REVIEW_NOTES"
                
                echo "Commit $FULL_COMMIT_ID review status updated to: $NEW_STATUS"
                if [ -n "$REVIEW_NOTES" ]; then
                    echo "Review notes saved."
                fi
                
                # Check if any PRs containing this commit are now ready for GitHub approval
                PR_UUIDS_FOR_COMMIT=$(task "project:local-ci.$PROJECT" "depends:$COMMIT_UUID" export 2>/dev/null | jq -r '.[] | select(.pr_number) | .uuid')
                for pr_uuid_check in $PR_UUIDS_FOR_COMMIT; do
                    if [ -n "$pr_uuid_check" ]; then
                        post_github_approval_if_ready "$pr_uuid_check"
                    fi
                done
                break
            else
                # Editor failed (e.g. user typed :cq in vim)
                rm "$TEMP_FILE"
                echo "Editor exited with error. Review cancelled."
                continue
            fi
        done
        ;;
    *)
        echo "Usage:"
        echo "   $0 list"
        echo "   $0 pr <number>"
        echo "   $0 nack <number>"
        echo "   $0 commit <commit_id> review"
        echo "   $0 commit <commit_id> reset-ci"
        exit 1
        ;;
esac
