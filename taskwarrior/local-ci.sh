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
        
        # Figure out where we are and query Github for the PR info.
        locate_repo
        JSON_DATA=$(gh pr view "$pr_num" --json commits,title,author | jq -c)
        
        # Extract PR data
        PR_TITLE=$(echo "$JSON_DATA" | jq -r .title)
        PR_AUTHOR=$(echo "$JSON_DATA" | jq -r .author.login)
        PR_COMMITS=$(echo "$JSON_DATA" | jq -r '.commits[].oid')
        
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
        PR_UUID=$(tw_upsert "${PR_FILTER[@]}" -- "${PR_UPSERT[@]}")

        # Now handle individual commit tasks
        echo "Processing commits for PR $pr_num..."
        while IFS= read -r commit_id; do
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
                task "$PR_UUID" modify "depends:$COMMIT_UUID"
            fi
        done <<< "$PR_COMMITS"

        if [ -n "$COMMIT_UUID" ]; then
            task "$COMMIT_UUID" modify +TIP_COMMIT
        fi
        
        echo "Finished processing PR $pr_num. Task UUID $PR_UUID"
        ;;
    run)
        run "${ARG_COMMAND_ARGS[@]}"
        ;;
    *)
        echo "Usage:"
        echo "   $0 list"
        exit 1
        ;;
esac
