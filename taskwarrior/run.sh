# Should be sourced from local-ci.sh.
# Not intended to be run directly.

run_ci_loop() {
    local backoff_sec=30
    local sleep_sec=$backoff_sec # trigger warning/status update on first iteration
    local filter=(project:local-ci claimedby: or "claimedby:$HOSTNAME")

    local busy=false
    local warnings=()

    while true; do
        # Find next approved commit that needs CI
        local next_commit_uuid=$(task "review_status:approved" "ci_status:unstarted" "commit_id.any:" export | jq -r '.[0].uuid // empty')

        # Do status echos. If we've been idle, sleep and periodically
        # output a "nothing to do" message.
        if [ "$busy" != "false" ]; then
            check_and_push_ready_prs
            backoff_sec=30
            sleep_sec=1
        else
            sleep 1
            ((sleep_sec++))

            if [ "$sleep_sec" -ge "$backoff_sec" ]; then
                    # Will max out at 480, 960, 1920, etc., whichever is greater than
                # the number written here. (64 minutes apparently.)
                if [ "$backoff_sec" -lt 2400 ]; then
                    backoff_sec=$((backoff_sec * 2))
                    sleep_sec=0
                fi

                check_and_push_ready_prs

                echo "([$(date +"%F %T")] Nothing to do. (Next message in $((backoff_sec / 60)) minutes.)"
                if [ ${#warnings[@]} -gt 0 ]; then
                    echo "Current warnings:" >&2
                    for warning in "${warnings[@]}"; do
                        echo "    $warning"
                    done
                    echo "(You need to Ctrl+C and restart local-ci.sh run to reset the warnings.)" >&2
                fi
                sleep_sec=1
            fi
        fi

        # Check whether there's anything to build this loop.
        if [ -z "$next_commit_uuid" ]; then
            busy=false
            continue
        fi
        busy=true

        # Get commit details
        local commit_data=$(task "$next_commit_uuid" export | jq -r '.[0]')
        local commit_id=$(echo "$commit_data" | jq -r '.commit_id')
        local repo_root=$(echo "$commit_data" | jq -r '.repo_root')
        local project=$(echo "$commit_data" | jq -r '.project')
        
        echo "Processing commit $commit_id from project $project"
        
        # Mark as started
        task "$next_commit_uuid" modify "ci_status:started"
        
        # Change to repo directory
        pushd "$repo_root" > /dev/null
        
        # Compute nixfile path
        local nixfile_path="$(
            echo "$project" | sed "s#^local-ci.#$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE/#"
        )".check-pr.nix
        if ! [ -f "$nixfile_path" ]; then
            warnings+=("Failing job for $commit_id with project $project since nixfile path is not found: $nixfile_path'")
            task "$next_commit_uuid" modify "ci_status:failed"
            popd > /dev/null
            continue
        fi
        
        # Check out local CI
        pushd "$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE" > /dev/null
        git reset --hard "$LOCAL_CI_COMMIT_ID"
        if [ -n "$LOCAL_CI_DIFF" ]; then
            echo "$LOCAL_CI_DIFF" | git apply --allow-empty
        fi
        popd > /dev/null
        
        # Compute cargoNix for Rust projects
        local cargo_nixes="{}"
        local lockfiles=($(git ls-tree -r --name-only "$commit_id" | grep "Cargo.*\.lock$" || true))
        
        if [ ${#lockfiles[@]} -gt 0 ]; then
            local cargo_nix_entries=()
            for lockfile in "${lockfiles[@]}"; do
                echo "Found Cargo.lock at $lockfile."
                # In theory this map should be "key = <path to Cargo.nix>" but I was unable to get
                # that setup to work, or to convince myself it was even faster than the current
                # setup, so instead the keys are used as lockfile paths and the values ignored.
                # See block comments in andrew-util.nix about this.
                cargo_nix_entries+=("\"$lockfile\" = null;")
            done
            
            if [ ${#cargo_nix_entries[@]} -gt 0 ]; then
                cargo_nixes="{ ${cargo_nix_entries[@]} }"
            fi
        fi
        
        # If we have Rust code but no lockfiles, that's an error
        if [ ${#lockfiles[@]} -eq 0 ] && git ls-tree -r --name-only "$commit_id" | grep -q "Cargo\.toml$"; then
            warnings+=("Failing job for $commit_id with project $project since Cargo.toml files found but no Cargo.lock files")
            task "$next_commit_uuid" modify "ci_status:failed"
            popd > /dev/null
            continue
        fi

        # Build commit string for nix
        local is_tip=$(task "$next_commit_uuid" export | jq -r '.[0].tags // [] | contains(["TIP_COMMIT"])')
        local commit_str="{
            commit = \"$commit_id\";
            isTip = $is_tip;
            gitUrl = \"$repo_root\";
            cargoNixes = $cargo_nixes;
        }"
        
        # Try to instantiate derivation
        local derivation_path
        if derivation_path=$(nix-instantiate \
            --arg inlineJsonConfig "{ gitDir = \"$repo_root\"; projectName = \"$project\"; }" \
            --arg inlineCommitList "[ $commit_str ]" \
            --argstr prNum "" \
            "$nixfile_path")
        then
            echo "Instantiated derivation: $derivation_path"
            task "$next_commit_uuid" modify "derivation:$derivation_path"
        else
            echo "Failed to instantiate derivation for commit $commit_id"
            task "$next_commit_uuid" modify "ci_status:failed"
            popd > /dev/null
            continue
        fi
        
        # Build the derivation
        echo "Building derivation for commit $commit_id..."
        if nix-build \
            --builders-use-substitutes \
            --no-build-output \
            --no-out-link \
            --keep-failed \
            --keep-derivations \
            --keep-outputs \
            --log-lines 100 \
            "$derivation_path" \
            --log-format internal-json -v \
            2> >(nom --json 2>/dev/null || cat >&2)
        then
            echo "Build succeeded for commit $commit_id"
            task "$next_commit_uuid" modify "ci_status:success"
            
            check_for_pushable_merges "$next_commit_uuid"
        else
            echo "Build failed for commit $commit_id"
            task "$next_commit_uuid" modify "ci_status:failed"
        fi
        
        popd > /dev/null
        echo "Finished processing commit $commit_id"
        echo
    done
}

check_and_push_ready_prs() {
    # Find PRs with merge_status:needsig
    local needsig_pr_uuids=$(task "merge_status:needsig" "pr_number.any:" export | jq -r '.[].uuid')
        
    for pr_uuid in $needsig_pr_uuids; do
        if [ -n "$pr_uuid" ]; then
            local pr_data=$(task "$pr_uuid" export | jq -r '.[0]')
            local pr_number=$(echo "$pr_data" | jq -r '.pr_number')
            local jj_change_id=$(echo "$pr_data" | jq -r '.jj_change_id // ""')
            local stored_base_commit=$(echo "$pr_data" | jq -r '.base_commit // ""')
            local repo_root=$(echo "$pr_data" | jq -r '.repo_root')
            local project=$(echo "$pr_data" | jq -r .project | sed "s/^local-ci.//")

            if [ -z "$jj_change_id" ]; then
                echo "$project PR #$pr_number has no JJ change ID, skipping push check"
                continue
            fi
                
            pushd "$repo_root" > /dev/null
                
            # Refresh base ref info from GitHub
            local fresh_json=$(gh pr view "$pr_number" --json baseRefName | jq -c)
            local base_ref=$(echo "$fresh_json" | jq -r '.baseRefName // "master"')
                
            # Fetch latest base ref
            git fetch origin "+refs/heads/$base_ref:refs/heads/pull/$pr_number/base" 2>/dev/null || true
            local fresh_base_commit=$(git rev-parse "pull/$pr_number/base" 2>/dev/null || echo "")
                
            if [ -z "$fresh_base_commit" ]; then
                echo "PR #$pr_number base ref $base_ref not found, resetting merge_status"
                task "$pr_uuid" modify "merge_status:unstarted" "base_commit:" "jj_change_id:"
                popd > /dev/null
                continue
            fi
                
            # Check if base commit changed
            if [ "$fresh_base_commit" != "$stored_base_commit" ]; then
                echo "⚠️  PR #$pr_number base commit changed from $stored_base_commit to $fresh_base_commit"
                echo "   The base branch has been updated. Please run 'local-ci.sh pr $pr_number info' to create a new merge commit."
                echo "   Resetting merge_status to unstarted."
                    
                # Remove old merge commit dependency
                local old_merge_uuids=$(task "project:local-ci.$PROJECT" "commit_id.any:" "+MERGE_COMMIT" export | jq -r --arg base "$stored_base_commit" '.[] | select(.description | contains($base)) | .uuid')
                for old_uuid in $old_merge_uuids; do
                    if [ -n "$old_uuid" ]; then
                        task "$pr_uuid" modify "depends:-$old_uuid"
                    fi
                done
                    
                # Reset PR status - user needs to recreate merge commit interactively
                task "$pr_uuid" modify "merge_status:unstarted" "base_commit:" "jj_change_id:"
                popd > /dev/null
                continue
            fi
            
            # Check if JJ change has GPG signature
            if command -v jj >/dev/null 2>&1; then
                local has_signature=$(jj log -r "$jj_change_id" --no-graph -T 'if(signature, "true", "false")' 2>/dev/null || echo "false")

                # Recompute description to get latest ACK set
                COMPUTE_MERGE_DESC="$LOCAL_CI_PATH/sqlite/compute_merge_description.py"
                if [ ! -f "$COMPUTE_MERGE_DESC" ]; then
                    echo "Error: compute_merge_description.py not found at $COMPUTE_MERGE_DESC" >&2
                    exit 1
                fi
                local description
                local ack_count
                description=$("$COMPUTE_MERGE_DESC" -c "$jj_change_id" "$pr_number" --no-acks-ok)
                # This is a little goofy; it would be better to use a real programming language and
                # to have compute_merge_description.py just return the list of ACKs. But okay.
                ack_count=$(echo "$description" | awk '/^ACKs for top commit:$/ {found=1} found && /^    .*ACK/' | wc -l)
                jj describe --quiet -r "$jj_change_id" -m "$description"
                                
                # Get the merge commit ID from JJ change
                local merge_commit_id=$(jj log --no-graph -r "$jj_change_id" -T commit_id 2>/dev/null || echo "")
                
                if [ "$has_signature" = "true" ]; then
                    echo "$project PR #$pr_number has GPG signature, pushing to $base_ref"
                    
                    if [ -n "$merge_commit_id" ] && git push origin "$merge_commit_id:$base_ref"; then
                        echo "Successfully pushed PR #$pr_number to $base_ref ($ack_count ACKs)"
                        task "$pr_uuid" modify "merge_status:pushed"
                    else
                        echo "Failed to push PR #$pr_number to $base_ref"
                    fi
                else
                    echo "$project PR #$pr_number JJ change $jj_change_id does not have GPG signature yet ($ack_count ACKs)"
                fi
            else
                echo "jj command not available, cannot check GPG signature for PR #$pr_number"
            fi
            
            popd > /dev/null
        fi
    done
}
