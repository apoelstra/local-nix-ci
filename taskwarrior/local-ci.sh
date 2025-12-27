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
    run)
        run "${ARG_COMMAND_ARGS[@]}"
        ;;
    *)
        echo "Usage:"
        echo "   $0 list"
        exit 1
        ;;
esac