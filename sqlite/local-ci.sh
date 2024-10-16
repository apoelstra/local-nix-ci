#!/usr/bin/env bash

###
### When stopping on 2024-10-15 -- try regenning db file (schema has changed) then continue working on run_command
### which is currently in the middle of implementing lookups of data
###

set -euo pipefail

command -v git >/dev/null 2>&1 || { echo "git is required but not installed. Aborting."; exit 1; }
command -v sqlite3 >/dev/null 2>&1 || { echo "sqlite3 is required but not installed. Aborting."; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "jq is required but not installed. Aborting."; exit 1; }
command -v send-text.sh >/dev/null 2>&1 || { echo "send-text.sh is required but not installed. Aborting."; exit 1; }

# Global setup
DB_FILE="$HOME/local-ci.db"
OUT_DIR=/nix/var/nix/gcroots/per-user/apoelstra/ci-output
NIX_PIN_PATH="$HOME/code/NixOS/nixpkgs/local-ci-pin/"

export NIX_PATH=nixpkgs=$NIX_PIN_PATH
NIXPKGS_COMMIT_ID=$(cd "$NIX_PIN_PATH/" && git rev-parse HEAD)
LOCAL_CI_PATH="$(cd $(dirname $(realpath "$0")); git rev-parse --show-toplevel)"
LOCAL_CI_WORKTREE="../local-ci-worktree"
LOCAL_CI_COMMIT_ID="$(cd $(dirname $(realpath "$0")); git rev-parse HEAD)"
LOCAL_CI_DIFF="$(cd $(dirname $(realpath "$0")); git diff)"

# Arguments (will be populated by parse_arguments)
ARG_COMMAND=
ARG_COMMAND_ARGS=()
ARG_REPO_NAME=
ARG_ALLOW_DIRTY_LOCAL_CI="no"

DB_REPO_ID=

if [ ! -e "$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE" ]; then
    echo "Warning: creating worktree at $LOCAL_CI_PATH/$LOCAL_CI_WORKTREE."
    echo "Any actions taken in this worktree are likely to be overwritten."
    pushd "$LOCAL_CI_PATH"
    git worktree add "$LOCAL_CI_WORKTREE"
    popd
fi

# Function to parse and handle --repo-dir argument
parse_arguments() {
    local args=("$@")
    for ((i = 0; i < ${#args[@]}; i++)); do
        if [[ "${args[i]}" == "--repo" ]] && (( i + 1 < ${#args[@]} )); then
            ARG_REPO_NAME="${args[i+1]}"
            ((i++)) # Skip next item since it's the directory
        elif [[ "${args[i]}" == "--allow-dirty-local-ci" ]]; then
            ARG_ALLOW_DIRTY_LOCAL_CI="yes"
        else
            if [ "$ARG_COMMAND" == "" ]; then
                ARG_COMMAND=${args[i]}
            else
                ARG_COMMAND_ARGS+=("${args[i]}") # Collect remaining arguments
            fi
        fi
    done
}

# Look up repository in the database.
locate_repo() {
    # First, find repo's git dir, by reading the command-line --repo-name and
    # otherwise just looking at the current git directory.
    if [ ! -z "$ARG_REPO_NAME" ]; then
        local escaped_repo_name="${ARG_REPO_NAME//\'/\'\'}"
        local num_results=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM repos WHERE name = '$escaped_repo_name';")
        if [ "$num_results" -eq 0 ]; then
            echo "No repository found with name '$ARG_REPO_NAME'."
            echo "Please initialize the repository using: $0 init-repo '$ARG_REPO_NAME'"
            exit 1
        elif [ "$num_results" -gt 1 ]; then
            echo "Multiple repositories found with name '$ARG_REPO_NAME'. Please check the database."
            exit 1
        else
            GIT_DIR=$(sqlite3 "$DB_FILE" "SELECT dot_git_path FROM repos WHERE name = '$escaped_repo_name'")
        fi
    # Then, if the user set GIT_DIR, use that
    elif [ -v GIT_DIR ]; then
        true
    # Otherwise, query git
    else
        if ! GIT_DIR="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
            echo "We do not appear to be running from a git repo and --repo was not provided."
            exit 1
        fi
    fi

    # Next, check whether this is in the database.
    local escaped_git_path="${GIT_DIR//\'/\'\'}"
    local num_results=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM repos WHERE dot_git_path = '$escaped_git_path';")

    # Check whether our git dir makes sense.
    if [ ! -d "$GIT_DIR" ]; then
        echo "Git directory $GIT_DIR appears not to be a directory. Are you in a git repo? In a workspace?"
        exit 1
    else
        if [ "$num_results" -eq 0 ]; then
            echo "No repository found with git directory '$GIT_DIR'."
            echo "Please initialize the repository using: $0 init-repo <repo name>'"
            exit 1
        elif [ "$num_results" -gt 1 ]; then
            echo "Multiple repositories found with git directory '$GIT_DIR'. Please check the database."
            exit 1
        fi
    fi

    # Success. Obtain repo ID and export GIT_DIR for use by git.
    DB_REPO_ID=$(sqlite3 "$DB_FILE" "SELECT id FROM repos WHERE dot_git_path = '$escaped_git_path';")
    export GIT_DIR
}

# init-repo command
init_repo() {
    local repo_name="${1-}"
    local nixfile_path="${2-}"
    if [ -z "$repo_name" ]; then
        echo "Error: Repository name is required."
        exit 1
    fi
    if [ -z "$nixfile_path" ]; then
        echo "Error: Nix file path is required. Available files:"
        pushd "$LOCAL_CI_PATH"
            ls *check-pr.nix
        popd
        exit 1
    fi

    if [ ! -f "$LOCAL_CI_PATH/$nixfile_path" ]; then
        echo "Error: Nix file path does not appear to be a file. Available files:"
        pushd "$LOCAL_CI_PATH"
            ls *check-pr.nix
        popd
        exit 1
    fi

    if ! dot_git_path="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
        echo "We do not appear to be running from a git repo."
        exit 1
    fi

    # Escape variables for SQL
    local escaped_repo_name="${repo_name//\'/\'\'}"
    local escaped_nixfile_path="${nixfile_path//\'/\'\'}"
    local escaped_git_path="${dot_git_path//\'/\'\'}"

    # Check if it is already in database
    local repo_count
    repo_count=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM repos WHERE name = '$escaped_repo_name' OR dot_git_path = '$escaped_git_path';")
    if [ "$repo_count" -ne 0 ]; then
        echo "Database already contains a repository with this name or git path."
        exit 1
    fi

    # Insert into database
    local origin_url
    if origin_url=$(git remote get-url origin); then
        local escaped_origin_url="${origin_url//\'/\'\'}"
        sqlite3 "$DB_FILE" "INSERT INTO repos (name, url, dot_git_path, nixfile_path) VALUES ('$escaped_repo_name', '$escaped_origin_url', '$escaped_git_path', '$nixfile_path');"
    else
        sqlite3 "$DB_FILE" "INSERT INTO repos (name, dot_git_path, nixfile_path) VALUES ('$escaped_repo_name', '$escaped_git_path', '$nixfile_path');"
    fi

    echo "Repository '$repo_name' initialized."
}

# Queue a new PR run
queue_pr() {
    locate_repo

    # First, sanity-check the PR number
    local pr_num="${1:-}"
    case $pr_num in
        '')
            echo "PR number is required by queue-pr command"
            exit 1
            ;;
        *[!0-9]*)
            echo "PR number must be a number, not $pr_num"
            exit 1
            ;;
    esac
    shift

    local on_success="COMMENT"
    if [ -n "${1:-}" ]; then
        if [ "$1" == "ACK" ]; then
            on_success="ACK"
            shift
        elif [ "$1" == "NOCOMMENT" ]; then
            on_success="NONE"
            shift
        else
            echo "Warning: not ACKing PR."
        fi
    fi
    local escaped_github_comment
    if [ -z "$@" ]; then
        escaped_github_comment="NULL"
    else
        escaped_github_comment="'${@//\'/\'\'}'"
    fi

    local head_commit
    local merge_commit

    if ! head_commit=$(git rev-parse "pr/$pr_num/head" 2>/dev/null); then
        echo "No commit at rev pr/$pr_num/head. Perhaps you need to fetch?"
        exit 1
    fi
    if ! merge_commit=$(git rev-parse "pr/$pr_num/merge" 2>/dev/null); then
        echo "No commit at rev pr/$pr_num/merge. Perhaps this PR was already merged?"
        exit 1
    fi

    # Then, obtain the list of commits
    local commits=($(git rev-list "$head_commit" --not "$merge_commit~"))
    local escaped_diff="${LOCAL_CI_DIFF//\'/\'\'}"

    echo "PR $pr_num has ${#commits[@]} commits"
    (
        cat <<EOF
BEGIN TRANSACTION;

INSERT INTO derivations (nixpkgs_commit, local_ci_commit, local_ci_diff, repo_id)
    VALUES ('$NIXPKGS_COMMIT_ID', '$LOCAL_CI_COMMIT_ID', '$escaped_diff', $DB_REPO_ID);

INSERT INTO tasks (task_type, pr_number, on_success, github_comment, repo_id, derivation_id)
    SELECT 'PR', $pr_num, '$on_success', $escaped_github_comment, $DB_REPO_ID, id FROM derivations WHERE id = last_insert_rowid();

INSERT INTO tasks_executions (task_id, time_queued)
    SELECT id, datetime('now') FROM tasks WHERE id = last_insert_rowid();
EOF

        for ((i = 0; i < ${#commits[@]}; i++)); do
            # Insert commit
            cat <<EOF
INSERT INTO task_commits (task_id, commit_id)
    SELECT id, '${commits[i]}' FROM tasks ORDER BY id DESC LIMIT 1;
EOF
            # Insert its lockfiles
            local lockfiles=($(git ls-tree -r --name-only ${commits[i]} | grep -e 'Cargo.*lock'))
            if [ "${#lockfiles[@]}" -ne 0 ]; then
                for ((j = 0; j < ${#lockfiles[@]}; j++)); do
                    local escaped_lockfile_name=${lockfiles[j]//\'/\'\'}
                    local lockfile_content=$(git show "${commits[i]}":"${lockfiles[j]}")
                    local lockfile_gitid=$(git rev-parse "${commits[i]}":"${lockfiles[j]}")
                    local lockfile_sha256=$(echo -n "$lockfile_content" | sha256sum | cut -d' ' -f1)
                    # Because the lockfile is in git, we don't need to store its contents in the db

                    # Insert the lockfile and its association with git commits
                    cat <<EOF
INSERT OR IGNORE INTO lockfiles (blob_id, full_text_sha2, name, repo_id)
    VALUES ('$lockfile_gitid', '$lockfile_sha256', '$escaped_lockfile_name', $DB_REPO_ID);

INSERT OR IGNORE INTO commit_lockfile (commit_id, lockfile_id)
    SELECT '${commits[i]}', id FROM lockfiles WHERE full_text_sha2 = '$lockfile_sha256';
EOF
                done
            else
                echo "Commit ${commits[i]} has no lockfiles in it. Bailing out. FIXME implement lockfile overrides" >&2
                exit 1
            fi
        done

        echo "COMMIT TRANSACTION;"
    ) | sqlite3 "$DB_FILE"
}

run_commands() {
    while true; do
        # Any changes to this SELECT must be mirrored below as a new local variable.
        local json_next_task
        json_next_task="$(sqlite3 -json "$DB_FILE" "
        SELECT
            tasks_executions.id AS execution_id,
            tasks_executions.task_id AS task_id,
            tasks_executions.status AS status,
            tasks.task_type AS task_type,
            tasks.derivation_id AS derivation_id,
            tasks.pr_number AS pr_number,
            derivations.local_ci_commit AS local_ci_commit,
            derivations.local_ci_diff AS local_ci_diff,
            derivations.path AS existing_derivation_path,
            derivations.time_instantiated AS existing_derivation_time,
            repos.name AS repo_name,
            repos.dot_git_path AS dot_git_path,
            repos.nixfile_path AS nixfile_path
        FROM
            tasks_executions
            JOIN tasks ON tasks_executions.task_id = tasks.id
            JOIN derivations ON tasks.derivation_id = derivations.id
            JOIN repos ON tasks.repo_id = repos.id
        WHERE
            tasks_executions.status = 'QUEUED'
            OR tasks_executions.status = 'IN PROGRESS'
        ORDER BY
            tasks_executions.time_queued
        DESC
        LIMIT 1;
        ")"

        # Check if a task was found
        if [ -z "$json_next_task" ] || [ "$json_next_task" == "[]" ]; then
            # No queued tasks, sleep and continue
            echo "(Nothing to do; sleeping 30 seconds.)"
            sleep 30
            continue
        fi

        local next_execution_id=$(echo "$json_next_task" | jq -r '.[0].execution_id')
        local next_task_id=$(echo "$json_next_task" | jq -r '.[0].task_id')
        local next_task_status=$(echo "$json_next_task" | jq -r '.[0].status')
        local task_type=$(echo "$json_next_task" | jq -r '.[0].task_type')
        local derivation_id=$(echo "$json_next_task" | jq -r '.[0].derivation_idempty')
        local pr_number=$(echo "$json_next_task" | jq -r '.[0].pr_number // empty')
        local local_ci_commit=$(echo "$json_next_task" | jq -r '.[0].local_ci_commit // empty')
        local existing_derivation_path=$(echo "$json_next_task" | jq -r '.[0].existing_derivation_path // empty')
        local existing_derivation_time=$(echo "$json_next_task" | jq -r '.[0].existing_derivation_time // empty')
        local repo_name=$(echo "$json_next_task" | jq -r '.[0].repo_name // empty')
        local dot_git_path=$(echo "$json_next_task" | jq -r '.[0].dot_git_path // empty')
        local nixfile_path=$(echo "$json_next_task" | jq -r '.[0].nixfile_path // empty')

        if [ -z "$dot_git_path" ]; then
            sleep 30
            continue
        fi

        if [ "$next_task_status" == "IN PROGRESS" ]; then
            echo "WARNING: contining in-progress job for PR $pr_number"
            echo "(Waiting 15 seconds to give time to Ctrl+C)"
            sleep 15
        fi

        case "$task_type" in
            PR)
                # From here on we are doing an execution.
                sqlite3 "$DB_FILE" "UPDATE tasks_executions SET status = 'IN PROGRESS', time_start = datetime('now') WHERE id = $next_execution_id;"
                # 1. If there is no existing derivation, instantiate one.
                if [ -z "$existing_derivation_path" ]; then
                    send-text.sh "Starting PR $pr_number (instantiating)."
                    # Check out local CI
                    pushd "$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE"
                    git reset --hard "$local_ci_commit"
                    echo "$json_next_task" | jq -r '.[0].local_ci_diff // empty' > patch.hmm
                    echo "$json_next_task" | jq -r '.[0].local_ci_diff // empty' | git apply --allow-empty

                    # Do instantiation
                    # FIXME for now we ignore the lockfiles and let nix figure it out

                    local isTip=true;
                    commits=()
                    for commit in $(sqlite3 "$DB_FILE" "SELECT commit_id FROM task_commits WHERE task_id = $next_task_id"); do
                        commits+=("{ commit = \"$commit\"; isTip = $isTip; gitUrl = $dot_git_path; }")
                        isTip=false
                    done
                    if existing_derivation_path=$(time nix-instantiate \
                        --arg jsonConfigFile false \
                        --arg inlineJsonConfig "{ gitDir = $dot_git_path; projectName = \"$repo_name\"; }" \
                        --arg inlineCommitList "[ $commits ]" \
                        --argstr prNum "$pr_number" \
                        -A checkPr \
                        "$nixfile_path")
                    then
                        local escaped_path=${existing_derivation_path//\'/\'\'}
                        sqlite3 "$DB_FILE" "UPDATE derivations SET path = '$escaped_path', time_instantiated = datetime('now') WHERE id = $derivation_id;"
                    else
                        sqlite3 "$DB_FILE" "UPDATE tasks_executions SET status = 'FAILED', time_end = datetime('now') WHERE id = $next_execution_id;"
                        send-text.sh "Instantiation of PR $pr_number failed."
                        sleep 60 # sleep 60 seconds to give me time to react if I am online
                        continue
                    fi
                else
                    send-text.sh "Starting PR $pr_number with existing drv $existing_derivation_path"
                fi
                # 2. Build the instantiated derivation
                if time nix-build \
                    --builders-use-substitutes \
                    --no-build-output \
                    --no-out-link \
                    --keep-failed \
                    --keep-derivations \
                    --keep-outputs \
                    --log-lines 100 \
                    "$existing_derivation_path" \
                    --log-format internal-json -v \
                    2> >(nom --json)
                then
                    sqlite3 "$DB_FILE" "UPDATE tasks_executions SET status = 'SUCCESS', time_end = datetime('now') WHERE id = $next_execution_id;"
                else
                    sqlite3 "$DB_FILE" "UPDATE tasks_executions SET status = 'FAILED', time_end = datetime('now') WHERE id = $next_execution_id;"
                    send-text.sh "Derivation of PR $pr_number failed: $existing_derivation_path"
                    sleep 60 # sleep 60 seconds to give me time to react if I am online
                    continue
                fi
                ;;
            *)
                echo "Don't know how to do task type $task_type yet."
                ;;
        esac
    done
}

# Main logic based on the command line argument
parse_arguments "$@"
if [[ "$ARG_ALLOW_DIRTY_LOCAL_CI" != "yes" ]] && [[ ! -z "$LOCAL_CI_DIFF" ]]; then
    echo "local CI directory apperas to be dirty and --allow-dirty-local-ci was not passed"
    exit 1
fi

if [ "$ARG_COMMAND" != "init-db" ]; then
    if [ -e "$DB_FILE" ]; then
        if [ ! -f "$DB_FILE" ]; then
            echo "Database file $DB_FILE appears not to be a file."
            echo "Please move it out of the way. Then to create a new database, run"
            echo "    $0 init-db"
            exit 1
        fi
    else
        echo "Database file $DB_FILE appears not to exist."
        echo "To create it, run"
        echo "    $0 init-db"
        exit 1
    fi
fi

case "$ARG_COMMAND" in
    init-db)
        SCHEMA="$(dirname $(realpath "$0"))/schema.txt"
        if [ ! -f "$SCHEMA" ]; then
            echo "Could not find schema file $SCHEMA."
            exit 1
        fi
        if [ -e "$DB_FILE" ]; then
            echo "Database file $DB_FILE appears to already exist."
            echo "Please move it out of the way first."
            exit 1
        fi
        set -x
        sqlite3 "$DB_FILE" < "$SCHEMA"
        ;;
    init-repo)
        init_repo "${ARG_COMMAND_ARGS[@]}"
        ;;
    queue-pr)
        queue_pr "${ARG_COMMAND_ARGS[@]}"
        ;;
    run)
        run_commands
        ;;
    *)
        echo "Usage: $0 {init-db | init-repo <repo-name> <nixfile-name> | queue-pr <pr #> [ACK] [comment] | run}"
        exit 1
        ;;
esac
