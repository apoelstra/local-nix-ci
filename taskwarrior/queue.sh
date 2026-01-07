# Should be sourced from local-ci.sh.
# Not intended to be run directly.

# Queue a run on a specific commit
#
# Arguments
#   ref -- git commit ID to test
#   parent -- (optional) UUID of task to set as parent of this
queue_commit() {
    locate_repo

    # First, sanity-check the PR number
    local ref="${1:-}"
    if [ -z "$ref" ]; then
        echo "git ref is required by queue-commit command"
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
    task add \
        "project:local-ci.$PROJECT" \
        "repo_root:$REPO_ROOT" \
        "commit_id:$commit" \
        "ci_status:unstarted" \
        "local-ci.sh queue-commit $ref"
}
