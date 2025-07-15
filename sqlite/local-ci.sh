#!/usr/bin/env bash

set -euo pipefail
shopt -s nullglob

command -v git >/dev/null 2>&1 || { echo "git is required but not installed. Aborting."; exit 1; }
command -v sqlite3 >/dev/null 2>&1 || { echo "sqlite3 is required but not installed. Aborting."; exit 1; }
command -v jj >/dev/null 2>&1 || { echo "jj is required but not installed. Aborting."; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "jq is required but not installed. Aborting."; exit 1; }
command -v send-text.sh >/dev/null 2>&1 || { echo "send-text.sh is required but not installed. Aborting."; exit 1; }
command -v github-merge.py >/dev/null 2>&1 || { echo "github-merge.py is required but not installed. Aborting."; exit 1; }

# Wrapper to catch "database is locked" errors and retry.
sqlite3() {
    local tries=5
    local delay=5
    local attempt rc
    local stderr_buf                                     # holds combined stdout/stderr each loop

    for (( attempt=1; attempt<=tries; ++attempt )); do
        # Capture *everything* so we can look for the lock error, but replay it faithfully.
        stderr_buf=$( command sqlite3 "$@" 2>&1 )
        rc=$?

        if (( rc == 0 )); then
            printf '%s' "$stderr_buf"   # was actually stdout; we captured both streams
            return 0
        fi

        if [[ $stderr_buf == *"database is locked"* ]] && (( attempt < tries )); then
            sleep "$delay"
            continue
        fi

        # Either a different error, or we’re out of retries
        printf '%s\n' "$stderr_buf" >&2
        return "$rc"
    done
}

# Global setup
DB_FILE="$HOME/local-ci.db"
NIX_PIN_PATH="$HOME/code/NixOS/nixpkgs/local-ci-pin/"

export NIX_PATH=nixpkgs=$NIX_PIN_PATH
export SEND_TEXT_APPLICATION="local-ci.sh on $(hostname)"
NIXPKGS_COMMIT_ID=$(cd "$NIX_PIN_PATH/" && git rev-parse HEAD)
LOCAL_CI_PATH="$(unset GIT_DIR; cd "$(dirname "$(realpath "$0")")"; git rev-parse --show-toplevel)"
LOCAL_CI_WORKTREE="../local-ci-worktree"
LOCAL_CI_COMMIT_ID="$(unset GIT_DIR; cd "$(dirname "$(realpath "$0")")"; git rev-parse HEAD)"
LOCAL_CI_DIFF="$(unset GIT_DIR; cd "$(dirname "$(realpath "$0")")"; git diff)"

# Arguments (will be populated by parse_arguments)
ARG_COMMAND=
ARG_COMMAND_ARGS=()
ARG_REPO_NAME=
ARG_ALLOW_DIRTY_LOCAL_CI="no"
ARG_REQUEUE="no"
QUEUE_PRIORITY=0

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
        elif [[ "${args[i]}" == "--priority" ]] && (( i + 1 < ${#args[@]} )); then
            QUEUE_PRIORITY="${args[i+1]}"
            if [[ ! "$QUEUE_PRIORITY" =~ ^[+-]?[1-9][0-9]*$ ]]; then
                echo "Priority $QUEUE_PRIORITY must be an integer without leading 0s."
                exit 2
            fi
            ((i++)) # Skip next item since it's the directory
        elif [[ "${args[i]}" == "--allow-dirty-local-ci" ]]; then
            ARG_ALLOW_DIRTY_LOCAL_CI="yes"
        elif [[ "${args[i]}" == "--requeue" ]]; then
            ARG_REQUEUE="yes"
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
    if [ -n "$ARG_REPO_NAME" ]; then
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
    if [ -z "$DB_REPO_ID" ]; then
        echo "Failed to find repo ID for git path $escaped_git_path"
        exit 1
    fi
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
            ls ./*check-pr.nix
        popd
        exit 1
    fi

    if [ ! -f "$LOCAL_CI_PATH/$nixfile_path" ]; then
        echo "Error: Nix file path does not appear to be a file. Available files:"
        pushd "$LOCAL_CI_PATH"
            ls ./*check-pr.nix
        popd
        exit 1
    fi

    if ! dot_git_path="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
        echo "We do not appear to be running from a git repo."
        exit 1
    fi

    # Escape variables for SQL
    local escaped_repo_name="${repo_name//\'/\'\'}"
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

echo_insert_rust_lockfiles() {
    local commit_id=$1
    if [ "$(git cat-file -t "$commit_id" 2>/dev/null)" != "commit" ]; then
        echo "echo_insert_rust_lockfiles: got bad commit ID $commit_id" >&2
        exit 2
    fi

    # Try using any lockfiles in the root of the repo. If there are none, try using
    # fallbacks found in the directory above the root.
    local lockfiles=($(git ls-tree --name-only $commit_id | grep '^Cargo.*\.lock$'))
    if [ "${#lockfiles[@]}" == 0 ]; then
        lockfiles=("$GIT_DIR"/../../*.lock); # note nullglob is on
        # If we have fallbacks, post a warning for each one. If not, no output -- we
        # will assume this isn't a Rust repo. If it is, the result may be a silently
        # empty test matrix. But we this is not the place to try to detect that.
        for ((j = 0; j < ${#lockfiles[@]}; j++)); do
            echo "$commit_id: Warning: using fallback lockfile ${lockfiles[j]}" >&2
        done
    fi

    echo "Commit $commit_id has ${#lockfiles[@]} lockfiles." >&2
    for ((j = 0; j < ${#lockfiles[@]}; j++)); do
        echo "Inserting lockfile $j: ${lockfiles[j]}" >&2

        local escaped_lockfile_name=${lockfiles[j]//\'/\'\'}
        local lockfile_content
        local lockfile_gitid
        local lockfile_gitpath
        local lockfile_sha256
        local nixfile

        if lockfile_gitid=$(git rev-parse --verify --quiet "$commit_id:${lockfiles[j]}" 2>/dev/null); then
            lockfile_content= #  not used
            lockfile_sha256=$(git show "$lockfile_gitid" | sha256sum | cut -d' ' -f1)
            lockfile_gitpath="$commit_id:${lockfiles[j]}"
        else
            lockfile_content=$(cat "${lockfiles[j]}")
            lockfile_sha256=$(echo -n "$lockfile_content" | sha256sum | cut -d' ' -f1)
            lockfile_gitpath="${lockfiles[j]}"
        fi

        nixfile=$("$LOCAL_CI_PATH/sqlite/create-cargo-nix.sh" \
          "$(git rev-parse --show-toplevel)" \
          "$commit_id" \
          "$lockfile_gitpath")
        if [ -z "$nixfile" ]
        then nixfile=NULL
        else nixfile="'$nixfile'"
        fi

        # Insert the lockfile and its association with git commits. Note use of INSERT OR IGNORE,
        # which in conjunction with the uniqueness constraint on `full_text_sha2`, will avoid
        # storing too much stuff
        cat <<EOF
INSERT OR IGNORE INTO lockfiles (blob_id, full_text_sha2, full_text, name, repo_id)
    VALUES ('$lockfile_gitid', '$lockfile_sha256', '$lockfile_content', '$escaped_lockfile_name', $DB_REPO_ID);

INSERT OR IGNORE INTO commit_lockfile (commit_id, lockfile_id, cargo_nix)
    SELECT '$commit_id', id, $nixfile FROM lockfiles WHERE full_text_sha2 = '$lockfile_sha256';
EOF
    done
}

# Queue a run on a specific commit
queue_commit() {
    locate_repo

    # First, sanity-check the PR number
    local ref="${1:-}"
    if [ -z "$ref" ]; then
        echo "git ref is required by queue-pr command"
        exit 1
    fi

    local commit
    if ! commit=$(git rev-parse "$ref" 2>/dev/null); then
        echo "No commit at ref $ref."
        exit 1
    fi
    shift

    local escaped_diff="${LOCAL_CI_DIFF//\'/\'\'}"

    echo "Queuing ref $ref; commit $commit"
    (
        cat <<EOF
BEGIN TRANSACTION;

INSERT INTO derivations (nixpkgs_commit, local_ci_commit, local_ci_diff, repo_id)
    VALUES ('$NIXPKGS_COMMIT_ID', '$LOCAL_CI_COMMIT_ID', '$escaped_diff', $DB_REPO_ID);

INSERT INTO tasks (task_type, on_success, github_comment, repo_id, derivation_id)
    SELECT 'PR', 'NONE', '', $DB_REPO_ID, id FROM derivations WHERE id = last_insert_rowid();

INSERT INTO tasks_executions (task_id, time_queued, priority)
    SELECT id, datetime('now'), $QUEUE_PRIORITY FROM tasks WHERE id = last_insert_rowid();

INSERT INTO task_commits (task_id, commit_id, is_tip)
    SELECT id, '$commit', 1 FROM tasks ORDER BY id DESC LIMIT 1;
EOF
        echo_insert_rust_lockfiles "$commit"
        echo "COMMIT TRANSACTION;"
    ) | sqlite3 "$DB_FILE"
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

    local on_success
    if [ -n "${1:-}" ]; then
        if [ "$1" == "ACK" ]; then
            on_success="ACK"
            shift
        elif [ "$1" == "COMMENT" ]; then
            echo "Warning: not ACKing PR." >&2
            on_success="COMMENT"
            shift
        elif [ "$1" == "NOCOMMENT" ]; then
            on_success="NONE"
            shift
        else
            echo "You must say ACK, COMMENT, or NOCOMMENT before your comment." >&2
            exit 1
        fi
    else
        echo "You must say ACK, COMMENT, or NOCOMMENT (then optionally a comment)." >&2
        exit 1
    fi
    local escaped_github_comment
    if [ "${#@}" -eq 0 ]; then
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

    echo "PR $pr_num has ${#commits[@]} commits; tip ${commits[0]}"
    (
        cat <<EOF
BEGIN TRANSACTION;

INSERT INTO derivations (nixpkgs_commit, local_ci_commit, local_ci_diff, repo_id)
    VALUES ('$NIXPKGS_COMMIT_ID', '$LOCAL_CI_COMMIT_ID', '$escaped_diff', $DB_REPO_ID);

INSERT INTO tasks (task_type, pr_number, on_success, github_comment, repo_id, derivation_id)
    SELECT 'PR', $pr_num, '$on_success', $escaped_github_comment, $DB_REPO_ID, id FROM derivations WHERE id = last_insert_rowid();

INSERT INTO tasks_executions (task_id, time_queued, priority)
    SELECT id, datetime('now'), $QUEUE_PRIORITY FROM tasks WHERE id = last_insert_rowid();
EOF

        local isTip=1;
        for ((i = 0; i < ${#commits[@]}; i++)); do
            echo "Adding commit $i: ${commits[i]}" >&2
            # Insert commit
            cat <<EOF
INSERT INTO task_commits (task_id, commit_id, is_tip)
    SELECT id, '${commits[i]}', $isTip FROM tasks ORDER BY id DESC LIMIT 1;
EOF
            echo_insert_rust_lockfiles "${commits[i]}"
            isTip=0
        done

        echo "COMMIT TRANSACTION;"
    ) | sqlite3 "$DB_FILE"
}

# Queue a new merge run
queue_merge() {
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

    # First fetch the current Github refs (this is also done in compute_merge_description.py
    # and in the bitcoin-maintainer-tools github-merge.py; it modifies the repo but in a way
    # that has never caused me problems in the years I've been using github-merge.py).
    #
    # Because it's weird and non-obvious, let me highlight: this creates a synthetic pr/X/base
    # ref, alongside pr/X/head and pr/X/merge (which are created by github) which points to
    # the "base ref" as obtained from Github by querying with gh.
    local base_ref
    base_ref=$(gh pr view "$pr_num" --json baseRefName --jq '.baseRefName')
    local remote=origin
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
    local local_merge_change_id
    local_merge_change_id=$(jj log --no-pager --no-graph -T change_id -r "latest(pull/$pr_num/head+ & pull/$pr_num/base+)")
    local_merge_change_id="${local_merge_change_id:0:12}" # truncate to 12 chars

    # If it conflicts, bail out
    if ! jj log --quiet -r "$local_merge_change_id & ~conflicts()" > /dev/null; then
        echo "Failed to queue $pr_num: conflict in merge change $local_merge_change_id"
        exit 1
    fi

    # Obtain its description and do initial sanity checks. If anything looks funny about
    # the PR (e.g. having @s in the commit message) the user will be given an opportunity
    # to bail here.
    local description
    local tree_hash
    if [ "$ARG_REQUEUE" = "yes" ]; then
        description=$("$LOCAL_CI_PATH/sqlite/compute_merge_description.py" --yes -c "$local_merge_change_id" "$pr_num")
    else
        description=$("$LOCAL_CI_PATH/sqlite/compute_merge_description.py" -c "$local_merge_change_id" "$pr_num")
    fi
    # Copy the tree hash out of the description to avoid computing it twice
    tree_hash=$(echo "$description" | grep "^Tree-SHA512: " | cut -d' ' -f2)

    # If we made it this far, the PR looks ok (or at least, the users says keep going).
    # Add the initial message and queue it for testing and for pushing.
    jj describe --quiet -r "$local_merge_change_id" -m "$description"
    local merge_commit
    merge_commit=$(jj log --no-graph -r "$local_merge_change_id" -T commit_id)

    local escaped_diff="${LOCAL_CI_DIFF//\'/\'\'}"
    echo "Queuing merge for PR $pr_num; merge change ID $local_merge_change_id commit ID $merge_commit"
    send-text.sh "Queue Merge" "PR $pr_num; merge change ID $local_merge_change_id commit ID $merge_commit"
    (
        cat <<EOF
BEGIN TRANSACTION;

INSERT INTO derivations (nixpkgs_commit, local_ci_commit, local_ci_diff, repo_id)
    VALUES ('$NIXPKGS_COMMIT_ID', '$LOCAL_CI_COMMIT_ID', '$escaped_diff', $DB_REPO_ID);

INSERT INTO tasks (task_type, pr_number, on_success, github_comment, repo_id, derivation_id)
    SELECT 'MERGE', $pr_num, 'NONE', NULL, $DB_REPO_ID, id FROM derivations WHERE id = last_insert_rowid();

INSERT INTO tasks_executions (task_id, time_queued, priority)
    SELECT id, datetime('now'), $QUEUE_PRIORITY FROM tasks WHERE id = last_insert_rowid();

INSERT INTO task_commits (task_id, commit_id, is_tip)
    SELECT id, '$merge_commit', 1 FROM tasks ORDER BY id DESC LIMIT 1;

INSERT INTO merge_pushes (repo_id, jj_change_id, tree_hash, target_branch, pr_number, state) VALUES
    ('$DB_REPO_ID', '$local_merge_change_id', '$tree_hash', '$base_ref', $pr_num, 'QUEUED');
EOF
        echo_insert_rust_lockfiles "$merge_commit"
        echo "COMMIT TRANSACTION;"
    ) | sqlite3 "$DB_FILE"
}

run_commands() {
    local backoff_sec=30
    local sleep_secs=$backoff_sec
    local should_send_signature_messages=true # send sig messages on startup

    local afk="";
    while true; do
        local last_afk=$afk
        local afk=$(echo "SELECT afk FROM config" | sqlite3 "$DB_FILE")
        if [ "$afk" != "$last_afk" ]; then
            echo "Away-from-keyboard: $afk" >&2
        fi

        if [ "$afk" = "AFK" ]; then
            local extra_order_by="tasks.task_type DESC,"
        else
            local extra_order_by=""
        fi
        # Any changes to this SELECT must be mirrored below as a new local variable.
        local json_next_task
        json_next_task="$(sqlite3 -json "$DB_FILE" "
        SELECT
            tasks_executions.id AS execution_id,
            tasks_executions.task_id AS task_id,
            tasks_executions.status AS status,
            tasks.on_success AS on_success,
            tasks.github_comment AS github_comment,
            tasks.task_type AS task_type,
            tasks.derivation_id AS derivation_id,
            tasks.pr_number AS pr_number,
            derivations.local_ci_commit AS local_ci_commit,
            derivations.local_ci_diff AS local_ci_diff,
            derivations.path AS existing_derivation_path,
            derivations.time_instantiated AS existing_derivation_time,
            repos.id AS repo_id,
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
            $extra_order_by
            tasks_executions.priority DESC,
            tasks_executions.time_queued ASC
        LIMIT 1;
        ")"

        # Function to mark queued MERGE tasks as failed and generate appropriate message
        mark_merge_tasks_failed_and_message() {
            local git_commit_id="$1"
            local base_message="$2"

            local failed_tasks=$(sqlite3 "$DB_FILE" "
                UPDATE tasks_executions
                SET status = 'FAILED', time_end = datetime('now')
                WHERE status = 'QUEUED'
                AND task_id IN (
                    SELECT t.id FROM tasks t
                    JOIN task_commits tc ON t.id = tc.task_id
                    WHERE t.task_type = 'MERGE' AND tc.commit_id = '$git_commit_id'
                );
                SELECT COUNT(*) FROM tasks_executions
                WHERE status = 'FAILED'
                AND time_end = datetime('now')
                AND task_id IN (
                    SELECT t.id FROM tasks t
                    JOIN task_commits tc ON t.id = tc.task_id
                    WHERE t.task_type = 'MERGE' AND tc.commit_id = '$git_commit_id'
                );")

            if [ "$failed_tasks" -gt 0 ]; then
                echo "$base_message and marking $failed_tasks queued MERGE tasks as failed."
            else
                echo "$base_message"
            fi
        }

        # Check merge_pushes table for any pending pushes
        local merge_push_messages=()
        local signature_messages=()
        local should_restart_loop=false

        # Get all repos with pending merge pushes
        local repos_with_pushes=$(sqlite3 "$DB_FILE" "
            SELECT DISTINCT r.id, r.name, r.dot_git_path
            FROM merge_pushes mp
            JOIN repos r ON mp.repo_id = r.id;")

        if [ -n "$repos_with_pushes" ]; then
            while IFS='|' read -r repo_id repo_name repo_git_path; do
                # Get the target remote for this repo
                local target_remote=$(sqlite3 "$DB_FILE" "SELECT DISTINCT target_remote FROM merge_pushes WHERE repo_id = $repo_id LIMIT 1;")

                # Fetch from the target remote to get latest changes
                pushd "$repo_git_path/.." > /dev/null
                git fetch "$target_remote" > /dev/null

                # Get all pending merge pushes for this repo
                local merge_pushes=$(sqlite3 "$DB_FILE" "
                    SELECT id, jj_change_id, tree_hash, pr_number, state
                    FROM merge_pushes
                    WHERE repo_id = $repo_id;")

                while IFS='|' read -r push_id jj_change_id tree_hash pr_num state; do
                    jj_change_id="${jj_change_id:0:12}" # truncate to 12 chars
                    if [ -n "$push_id" ]; then
                        # First, check if the change has been abandoned, since nothing else will work
                        # if the change is gone.
                        local git_commit_id
                        local current_tree_hash
                        if ! git_commit_id=$(jj log -r "$jj_change_id" --no-graph -T commit_id); then
                            sqlite3 "$DB_FILE" "DELETE FROM merge_pushes WHERE id = $push_id;"
                            # Cannot call mark_merge_tasks_failed_and_message because we don't know the commit ID;
                            # this can be obtained from jj but it's pretty hard -- we have to repeat the `jj log`
                            # call with `--at-operation <op>` where `<op>` is the ID *prior* to the abandonment
                            # in the op log. We can list all the op log IDs with something like
                            #
                            #    `jj op log --no-graph -T 'id ++ " " ++ description  ++ "\n"`
                            #
                            # but the description will be "abandon commit <commit ID>" so we gotta inspect each
                            # such entry to find the one that affected our *change id* most recently.
                            #
                            # In practice I expect I will only be manually abandoning commits after the tests have
                            # completed, since that's when it'll ping me to sign, so there won't be any queued
                            # tests to drop. So no big deal. But if I decide I want to clean up queued tasks,
                            # probably I will need to add the commit ID to the `merge_pushes` table and figure
                            # out how to keep it up-to-date.
                            merge_push_messages+=("Change $jj_change_id ($repo_name PR $pr_num) seems to have been abandoned. Removing from queue")
                            continue
                        fi

                        # Then, recompute and update description. Then we can do checks.
                        local description
                        local new_tree_hash
                        if ! description=$("$LOCAL_CI_PATH/sqlite/compute_merge_description.py" -y -c "$jj_change_id" "$pr_num"); then
                            echo "Failed to compute description for merge of $repo_id PR $pr_num."
                            echo "Typically this means that the PR has already been merged."

                            sqlite3 "$DB_FILE" "DELETE FROM merge_pushes WHERE id = $push_id;"
                            merge_push_messages+=("$(mark_merge_tasks_failed_and_message "$git_commit_id" "Cannot compute description for merge commit of $repo_name PR $pr_num. Change id $jj_change_id. Removing from queue.")")
                            continue
                        fi
                        # Copy the tree hash out of the description to avoid computing it twice
                        new_tree_hash=$(echo "$description" | grep "^Tree-SHA512: " | cut -d' ' -f2)

                        # a. Update our view of upstream and check that our merge commit still has
                        #    the PR's head as a parent. If not, the PR was updated and we should
                        #    cancel the merge.
                        local target_remote=$(sqlite3 "$DB_FILE" "SELECT target_remote FROM merge_pushes WHERE id = $push_id;")
                        git fetch -q "$target_remote" "+refs/pull/$pr_num/head:refs/heads/pull/$pr_num/head"
                        if ! jj log --quiet -r "$jj_change_id- & pull/$pr_num/head" > /dev/null; then
                            sqlite3 "$DB_FILE" "DELETE FROM merge_pushes WHERE id = $push_id;"
                            merge_push_messages+=("$(mark_merge_tasks_failed_and_message "$git_commit_id" "$repo_name PR $pr_num has been updated (change ID $jj_change_id no longer has pull/$pr_num/head as a parent). Removing from queue")")
                            continue
                        fi

                        # b. Update description (should pass since the commit was not abandoned,
                        #    but might not if something weird is going on)
                        if jj describe --quiet -r "$jj_change_id" -m "$description"; then
                            # c. Check if tree hash has changed
                            if [ "$new_tree_hash" != "$tree_hash" ]; then
                                sqlite3 "$DB_FILE" "DELETE FROM merge_pushes WHERE id = $push_id;"
                                merge_push_messages+=("$(mark_merge_tasks_failed_and_message "$git_commit_id" "Tree hash for change $jj_change_id ($repo_name PR $pr_num) has changed ($tree_hash → $new_tree_hash). Removing from queue")")
                                continue
                            fi

                            # d. Check if it's a direct descendant of the target branch
                            local target_branch=$(sqlite3 "$DB_FILE" "SELECT target_branch FROM merge_pushes WHERE id = $push_id;")
                            git fetch "$target_remote"
                            if ! git merge-base --is-ancestor "$target_remote/$target_branch" "$git_commit_id" 2>/dev/null; then
                                sqlite3 "$DB_FILE" "DELETE FROM merge_pushes WHERE id = $push_id;"
                                merge_push_messages+=("$(mark_merge_tasks_failed_and_message "$git_commit_id" "Change $jj_change_id ($repo_name PR $pr_num) is not a direct descendant of $target_remote/$target_branch. Removing from queue; attempted to requeue.")")
                                # The above lines deleted the queued merge. But the reason for doing so was that
                                # upstream master changed, while the PR itself did not change (if it did, we would
                                # have noticed it in an earlier check). So we should re-queue this.
                                #
                                # If requeuing failed the user will get a message from the `queue-merge` call, which
                                # will occur *before* `merge_push_messages` is shown.
                                if "$LOCAL_CI_PATH/sqlite/local-ci.sh" queue-merge --requeue "$pr_num"; then
                                    merge_push_messages+=("...Removing from queue; successfully requeued.")
                                else
                                    merge_push_messages+=("...Removing from queue; failed to requeue.")
                                fi
                                continue
                            fi

                            # e. Check if state is FAILED
                            if [ "$state" = "FAILED" ]; then
                                sqlite3 "$DB_FILE" "DELETE FROM merge_pushes WHERE id = $push_id;"
                                merge_push_messages+=("$(mark_merge_tasks_failed_and_message "$git_commit_id" "Test for change $jj_change_id ($repo_name PR $pr_num) failed. Removing from queue")")
                                continue
                            fi

                            # f. Check if state is SUCCESS and if it's signed
                            if [ "$state" = "SUCCESS" ]; then
                                if git verify-commit "$git_commit_id" &>/dev/null; then
                                    # Push the change
                                    if git push "$target_remote" "$git_commit_id:$target_branch"; then
                                        merge_push_messages+=("Change $jj_change_id ($repo_name PR $pr_num) was successfully pushed to $target_branch.")
                                    else
                                        merge_push_messages+=("Failed to push change $jj_change_id ($repo_name PR $pr_num) to $target_branch.")
                                    fi
                                    sqlite3 "$DB_FILE" "DELETE FROM merge_pushes WHERE id = $push_id;"
                                    should_restart_loop=true
                                else
                                    signature_messages+=("Change $jj_change_id ($repo_name PR $pr_num) is ready to be pushed but is not GPG signed.")
                                fi
                            fi
                        else
                            # `jj describe` failed, presumably because the change was already pushed and became immutable
                            sqlite3 "$DB_FILE" "DELETE FROM merge_pushes WHERE id = $push_id;"
                            merge_push_messages+=("$(mark_merge_tasks_failed_and_message "$git_commit_id" "Failed to update description for $jj_change_id ($repo_name PR $pr_num) -- maybe already pushed? Removing from queue")")
                        fi
                    fi
                done <<< "$merge_pushes"

                popd > /dev/null
            done <<< "$repos_with_pushes"

            # Send all action messages in a single call
            if [ ${#merge_push_messages[@]} -gt 0 ]; then
                send-text.sh "Push-queue" "$(printf "%s\n" "${merge_push_messages[@]}")"
            fi

            # Send signature messages only if we should
            if [ ${#signature_messages[@]} -gt 0 ] && [ "$should_send_signature_messages" = true ]; then
                send-text.sh "Push-queue (NEED SIGNATURES)" "$(printf "%s\n" "${signature_messages[@]}")"
                should_send_signature_messages=false
            fi

            # Restart the loop if we pushed something
            if [ "$should_restart_loop" = true ]; then
                continue
            fi
        fi

        # Check if a task was found
        if [ -z "$json_next_task" ] || [ "$json_next_task" == "[]" ]; then
            # No queued tasks, sleep and continue
            if [ "$sleep_secs" -ge "$backoff_sec" ]; then
                # Will max out at 480, 960, 1920, etc., whichever is greater than
                # the number written here. (32 minutes apparently.)
                if [ "$backoff_sec" -lt 1200 ]; then
                    backoff_sec=$((backoff_sec * 2))
                    sleep_secs=0
                fi

                echo "([$(date +"%F %T")] Nothing to do. (Next message in $((backoff_sec / 60)) minutes.)"
                # Output signature messages on "nothing to do" messages
                if [ ${#signature_messages[@]} -gt 0 ]; then
                    echo "$(printf "%s\n" "${signature_messages[@]}")"
                    should_send_signature_messages=false
                fi
                sleep_secs=0;
            fi

            sleep 5
            sleep_secs=$((sleep_secs + 5))

            # Update DB to signal inactivity
            SINCE=$(echo "SELECT inactive_since FROM config" | sqlite3 "$DB_FILE")
            if [ -z "$SINCE" ]; then
                echo "UPDATE config SET inactive_since = '$(date '+%F %T')'" | sqlite3 "$DB_FILE"
            # "sleep_secs" = "5" is a hacky way of checking whether we echoed the
            # "Nothing to do" message above.
            elif [ "$sleep_secs" = "5" ]; then
                echo "Inactive since $SINCE."
            fi

            continue
        else
            # Update DB to signal activity
            echo "UPDATE config SET inactive_since = ''" | sqlite3 "$DB_FILE"
            backoff_sec=15
            # Reset the flag to send signature messages next time
            should_send_signature_messages=true
        fi

        local next_execution_id=$(echo "$json_next_task" | jq -r '.[0].execution_id')
        local next_task_id=$(echo "$json_next_task" | jq -r '.[0].task_id')
        local next_task_status=$(echo "$json_next_task" | jq -r '.[0].status')
        local on_success=$(echo "$json_next_task" | jq -r '.[0].on_success')
        local github_comment=$(echo "$json_next_task" | jq -r '.[0].github_comment // empty')
        local task_type=$(echo "$json_next_task" | jq -r '.[0].task_type')
        local derivation_id=$(echo "$json_next_task" | jq -r '.[0].derivation_id')
        local pr_number=$(echo "$json_next_task" | jq -r '.[0].pr_number // empty')
        local local_ci_commit=$(echo "$json_next_task" | jq -r '.[0].local_ci_commit')
        local existing_derivation_path=$(echo "$json_next_task" | jq -r '.[0].existing_derivation_path // empty')
        local existing_derivation_time=$(echo "$json_next_task" | jq -r '.[0].existing_derivation_time // empty')
        local repo_id=$(echo "$json_next_task" | jq -r '.[0].repo_id')
        local repo_name=$(echo "$json_next_task" | jq -r '.[0].repo_name')
        local dot_git_path=$(echo "$json_next_task" | jq -r '.[0].dot_git_path')
        local nixfile_path="$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE/$(echo "$json_next_task" | jq -r '.[0].nixfile_path')"

        if [ -z "$dot_git_path" ]; then
            sleep 30
            continue
        fi

        if [ "$next_task_status" == "IN PROGRESS" ]; then
            echo "WARNING: contining in-progress $task_type job $next_task_id for $repo_name PR $pr_number"
            echo "(Waiting 15 seconds to give time to Ctrl+C)"
            sleep 15
        fi

        echo_commit_str() {
            local commit_id=$1
            local isTip=$2

            local lockfile_data
            lockfile_data="$(sqlite3 -json "$DB_FILE" "
            SELECT
                name,
                cargo_nix
            FROM
                lockfiles
                JOIN commit_lockfile ON commit_lockfile.lockfile_id = lockfiles.id
            WHERE
                commit_lockfile.commit_id = '$commit_id'
            ")"

            cat <<EOF
{
    commit = "$commit_id";
    isTip = $isTip;
    gitUrl = $dot_git_path;
    cargoNixes = { $(echo "$lockfile_data" | jq -r '.[] | "\"" + .name + "\" = " + .cargo_nix + ";"') };
}
EOF
        }

        commits=()
        local tip_commit
        for data in $(sqlite3 -separator '-' "$DB_FILE" "SELECT commit_id, is_tip FROM task_commits WHERE task_id = $next_task_id"); do
            local commit_id
            local isTip=false

            commit_id=$(echo "$data" | cut -d'-' -f1)
            if [ "$(echo "$data" | cut -d'-' -f2)" == "1" ]; then
              tip_commit=$commit_id
              isTip=true
            fi
            commits+=("$(echo_commit_str "$commit_id" "$isTip")")
        done
        sqlite3 "$DB_FILE" "UPDATE tasks_executions SET status = 'IN PROGRESS', time_start = datetime('now') WHERE id = $next_execution_id;"
        local strcommits="${commits[*]}"

        # From here on we are doing an execution.
        # 1. If there is no existing derivation, instantiate one.
        if [ -z "$existing_derivation_path" ]; then
            send-text.sh "Starting CI run" "Starting $repo_name $task_type $pr_number (instantiating)"
            # Check out local CI
            pushd "$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE"
            git reset --hard "$local_ci_commit"
            echo "$json_next_task" | jq -r '.[0].local_ci_diff // empty' | git apply --allow-empty

            # Do instantiation
            if existing_derivation_paths=$(time nix-instantiate \
                --arg inlineJsonConfig "{ gitDir = $dot_git_path; projectName = \"$repo_name\"; }" \
                --arg inlineCommitList "[ $strcommits ]" \
                --argstr prNum "$pr_number" \
                "$nixfile_path")
            then
                for existing_derivation_path in $existing_derivation_paths; do
                    local escaped_path=${existing_derivation_path//\'/\'\'}
                    sqlite3 "$DB_FILE" "UPDATE derivations SET path = '$escaped_path', time_instantiated = datetime('now') WHERE id = $derivation_id;"
                done
                popd
            else
                sqlite3 "$DB_FILE" "UPDATE tasks_executions SET status = 'FAILED', time_end = datetime('now') WHERE id = $next_execution_id;"
                send-text.sh "End CI run (fail)" "Instantiation of $repo_name PR $pr_number failed."
                popd
                echo "(Waiting 60 seconds (from $(date)) to give time to react.)"
                sleep 60 # sleep 60 seconds to give me time to react if I am online
                continue
            fi
        else
            send-text.sh "Starting CI run (continuation)" "Starting $repo_name $task_type $pr_number with existing drv $existing_derivation_path"
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
            if [ "$task_type" = "PR" ]; then
                if [ -n "$github_comment" ]; then
                    message="successfully ran local tests; $github_comment"
                else
                    message="successfully ran local tests"
                fi
                pushd "$dot_git_path/..";
                case $on_success in
                    ACK)
                        if ! gh pr review "$pr_number" -a -b "ACK ${tip_commit}; $message"; then
                            send-text.sh "Failed to ACK PR" "Failed to ACK PR $pr_number -- posting comment instead. (Is this your own PR?)"
                            gh pr review "$pr_number" -c -b "On ${tip_commit} $message"
                        fi
                        ;;
                    COMMENT)
                        gh pr review "$pr_number" -c -b "On ${tip_commit} $message"
                        ;;
                    NONE)
                        ;;
                esac
                popd
            fi

            # Set "SUCCESS" as the last step
            sqlite3 "$DB_FILE" "UPDATE tasks_executions SET status = 'SUCCESS', time_end = datetime('now') WHERE id = $next_execution_id;"

            # Check if this commit has a jj change ID in the merge_pushes table
            local jj_change_ids=$(sqlite3 "$DB_FILE" "
                SELECT jj_change_id FROM merge_pushes
                WHERE repo_id = $repo_id
                AND state = 'QUEUED'
                AND jj_change_id IN (
                    SELECT jj_change_id FROM merge_pushes
                    WHERE repo_id = $repo_id
                );")

            if [ -n "$jj_change_ids" ]; then
                pushd "$dot_git_path/.."
                local tip_change_id
                tip_change_id=$(jj log -r "$tip_commit" --no-graph -T change_id)
                tip_change_id="${tip_change_id:0:12}" # truncate to 12 chars
                for jj_change_id in $jj_change_ids; do
                    if [ "$jj_change_id" = "$tip_change_id" ]; then
                        sqlite3 "$DB_FILE" "UPDATE merge_pushes SET state = 'SUCCESS' WHERE jj_change_id = '$jj_change_id' AND repo_id = $repo_id;"
                    fi
                done
                popd
            fi

            send-text.sh "End CI run (success)" "Test of $repo_name $task_type $pr_number succeeded. Derivation: $existing_derivation_path"
        else
            sqlite3 "$DB_FILE" "UPDATE tasks_executions SET status = 'FAILED', time_end = datetime('now') WHERE id = $next_execution_id;"

            # Check if this commit has a jj change ID in the merge_pushes table
            local jj_change_ids=$(sqlite3 "$DB_FILE" "
                SELECT jj_change_id FROM merge_pushes
                WHERE repo_id = $repo_id
                AND state = 'QUEUED'
                AND jj_change_id IN (
                    SELECT jj_change_id FROM merge_pushes
                    WHERE repo_id = $repo_id
                );")

            if [ -n "$jj_change_ids" ]; then
                for jj_change_id in $jj_change_ids; do
                    # Check if this commit corresponds to the jj change ID
                    pushd "$dot_git_path/.."
                    if jj_commit_id=$(jj log -r "$jj_change_id" --no-graph -T commit_id 2>/dev/null); then
                        if [ "$jj_commit_id" = "$tip_commit" ]; then
                            sqlite3 "$DB_FILE" "UPDATE merge_pushes SET state = 'FAILED' WHERE jj_change_id = '$jj_change_id' AND repo_id = $repo_id;"
                        fi
                    fi
                    popd
                done
            fi

            send-text.sh "End CI run (fail)" "Test of $repo_name PR $pr_number failed: $existing_derivation_path"
            sleep 60 # sleep 60 seconds to give me time to react if I am online
            continue
        fi
    done
}

# Main logic based on the command line argument
parse_arguments "$@"
if [[ "$ARG_ALLOW_DIRTY_LOCAL_CI" != "yes" ]] && [[ ! -z "$LOCAL_CI_DIFF" ]]; then
    echo "local CI directory appears to be dirty and --allow-dirty-local-ci was not passed"
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
    afk)
        AFK=$(echo "SELECT afk FROM config" | sqlite3 "$DB_FILE")
        echo "Setting away-from-keyboard to AFK (was $AFK)."
        echo "UPDATE config SET afk = 'AFK'" | sqlite3 "$DB_FILE"
        ;;
   back)
        AFK=$(echo "SELECT afk FROM config" | sqlite3 "$DB_FILE")
        echo "Setting away-from-keyboard to BACK (was $AFK)."
        echo "UPDATE config SET afk = 'BACK'" | sqlite3 "$DB_FILE"
        ;;
    clear-queue)
        (cat <<EOF
-- This entire blurb from ChatGPT 2024-10-16
BEGIN TRANSACTION;

-- Step 1: Create temporary tables to store identifiers of records to be deleted

-- Temporary table for task_ids
CREATE TEMPORARY TABLE temp_task_ids AS
SELECT DISTINCT task_id FROM tasks_executions WHERE status IN ('QUEUED', 'IN PROGRESS');

-- Temporary table for derivation_ids
CREATE TEMPORARY TABLE temp_derivation_ids AS
SELECT DISTINCT derivation_id FROM tasks WHERE id IN (SELECT task_id FROM temp_task_ids);

-- Temporary table for commit_ids
CREATE TEMPORARY TABLE temp_commit_ids AS
SELECT DISTINCT commit_id FROM task_commits WHERE task_id IN (SELECT task_id FROM temp_task_ids);

-- Temporary table for lockfile_ids
CREATE TEMPORARY TABLE temp_lockfile_ids AS
SELECT DISTINCT lockfile_id FROM commit_lockfile
WHERE commit_id IN (SELECT commit_id FROM temp_commit_ids);

-- Step 2: Delete child records first

-- Delete from commit_lockfile
DELETE FROM commit_lockfile
WHERE commit_id IN (SELECT commit_id FROM temp_commit_ids);

-- Delete from task_commits
DELETE FROM task_commits
WHERE task_id IN (SELECT task_id FROM temp_task_ids);

-- Step 3: Delete parent records

-- Delete from tasks_executions
DELETE FROM tasks_executions
WHERE task_id IN (SELECT task_id FROM temp_task_ids);

-- Delete from tasks
DELETE FROM tasks
WHERE id IN (SELECT task_id FROM temp_task_ids);

-- Delete from derivations
DELETE FROM derivations
WHERE id IN (SELECT derivation_id FROM temp_derivation_ids);

-- Step 4: Clean up lockfiles that are no longer referenced

-- Identify lockfiles no longer referenced
CREATE TEMPORARY TABLE temp_unused_lockfile_ids AS
SELECT id FROM lockfiles
WHERE id IN (SELECT lockfile_id FROM temp_lockfile_ids)
AND id NOT IN (SELECT lockfile_id FROM commit_lockfile);

-- Delete from lockfiles
DELETE FROM lockfiles
WHERE id IN (SELECT id FROM temp_unused_lockfile_ids);

-- Step 5: Clean up commits that are no longer referenced

-- If you have a commits table and want to delete commits no longer referenced
-- CREATE TEMPORARY TABLE temp_unused_commit_ids AS
-- SELECT id FROM commits
-- WHERE id IN (SELECT commit_id FROM temp_commit_ids)
-- AND id NOT IN (SELECT commit_id FROM task_commits);

-- DELETE FROM commits
-- WHERE id IN (SELECT id FROM temp_unused_commit_ids);

-- Step 6: Drop temporary tables

DROP TABLE temp_task_ids;
DROP TABLE temp_derivation_ids;
DROP TABLE temp_commit_ids;
DROP TABLE temp_lockfile_ids;
DROP TABLE temp_unused_lockfile_ids;
-- DROP TABLE temp_unused_commit_ids;  -- If used

COMMIT;
EOF
) | sqlite3 "$DB_FILE"
        ;;
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
        sqlite3 "$DB_FILE" < "$SCHEMA"
        ;;
    init-repo)
        init_repo "${ARG_COMMAND_ARGS[@]}"
        ;;
    queue-commit)
        queue_commit "${ARG_COMMAND_ARGS[@]}"
        ;;
    queue-pr)
        queue_pr "${ARG_COMMAND_ARGS[@]}"
        ;;
    queue-merge)
        queue_merge "${ARG_COMMAND_ARGS[@]}"
        ;;
    run)
        run_commands
        ;;
    show-queue)
        AFK=$(echo "SELECT afk FROM config" | sqlite3 "$DB_FILE")
        TIME="${ARG_COMMAND_ARGS[0]:-24 hours ago}"
        echo "Away-from-keyboard: $AFK" >&2
        if [ "$AFK" = "AFK" ]; then
            extra_order_by="tasks.task_type"
        else
            extra_order_by="''"
        fi

        adayago=$(date '+%F %T' -d "$TIME")

        # Just output the raw json. It looks reasonable for human consumption
        # and is useful for machine consumption.
        sqlite3 -json "$DB_FILE" "
        SELECT
            tasks_executions.id AS id,
            repos.name AS repo_name,
            tasks.pr_number AS pr_number,
            tasks.task_type AS task_type,
            tasks_executions.status AS status,
            derivations.path AS derivation_path,
            tasks.on_success AS on_success,
            tasks.github_comment AS github_comment,
            tasks_executions.time_queued AS time_queued,
            tasks_executions.time_start AS time_started,
            tasks_executions.time_end AS time_ended
        FROM
            tasks_executions
            JOIN tasks ON tasks_executions.task_id = tasks.id
            JOIN derivations ON tasks.derivation_id = derivations.id
            JOIN repos ON tasks.repo_id = repos.id
        WHERE
            status = 'QUEUED'
            OR status = 'IN PROGRESS'
            OR time_end > '$adayago'
        ORDER BY
            CASE
                WHEN status IN ('SUCCESS', 'FAILED') THEN 0
                WHEN status = 'IN PROGRESS' THEN 1
                ELSE 2
            END,
            CASE
                WHEN status = 'QUEUED' THEN $extra_order_by
                ELSE ''
            END DESC,
            tasks_executions.priority DESC,
            CASE
                WHEN status IN ('SUCCESS', 'FAILED') THEN time_ended
                WHEN status = 'IN PROGRESS' THEN time_started
                ELSE time_queued
            END ASC
        " | jq
        ;;
    *)
        echo "Usage: $0 {init-db | init-repo <repo-name> <nixfile-name> | queue-commit <ref> | queue-pr <pr #> [ACK] [comment] | run}"
        exit 1
        ;;
esac
