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

        if [ -z "$next_commit_uuid" ]; then
            # Done work. If we did something, reset the sleep counter and
            # loop around. If we did nothing, sleep and periodically
            # output a "nothing to do" message.
            if [ "$busy" != "false" ]; then
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
        local lockfiles=($(git ls-tree -r --name-only "$commit_id" | grep "Cargo\.lock$" || true))
        
        if [ ${#lockfiles[@]} -gt 0 ]; then
            local cargo_nix_entries=()
            for lockfile in "${lockfiles[@]}"; do
                echo "Found Cargo.lock at $lockfile, generating Cargo.nix..."
                local cargo_nix_path
                if cargo_nix_path=$("$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE/taskwarrior/create-cargo-nix.sh" "$(pwd)" "$commit_id" "$lockfile" 2>/dev/null); then
                    local lockfile_key=$(echo "$lockfile" | sed 's/[^a-zA-Z0-9_]/_/g')
                    cargo_nix_entries+=("\"$lockfile_key\" = \"$cargo_nix_path\"")
                    echo "Generated Cargo.nix for $lockfile: $cargo_nix_path"
                else
                    echo "Failed to generate Cargo.nix for $lockfile"
                    warnings+=("Failed to generate Cargo.nix for $lockfile in commit $commit_id")
                    task "$next_commit_uuid" modify "ci_status:failed"
                    popd > /dev/null
                    continue 2
                fi
            done
            
            if [ ${#cargo_nix_entries[@]} -gt 0 ]; then
                cargo_nixes="{ $(IFS='; '; echo "${cargo_nix_entries[*]}") }"
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
            gitUrl = \"$(git rev-parse --git-dir)\";
            cargoNixes = $cargo_nixes;
        }"
        
        # Try to instantiate derivation
        local derivation_path
        if derivation_path=$(nix-instantiate \
            --arg inlineJsonConfig "{ gitDir = \"$(git rev-parse --git-dir)\"; projectName = \"$project\"; }" \
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
        
        # Check for PRs ready to push
        check_and_push_ready_prs
    done
}

check_and_push_ready_prs() {
    # Find PRs with merge_status:needsig
    local needsig_pr_uuids=$(task "merge_status:needsig" "pr_number.any:" export | jq -r '.[].uuid')
    
    while IFS= read -r pr_uuid; do
        if [ -n "$pr_uuid" ]; then
            local pr_data=$(task "$pr_uuid" export | jq -r '.[0]')
            local pr_number=$(echo "$pr_data" | jq -r '.pr_number')
            local jj_change_id=$(echo "$pr_data" | jq -r '.jj_change_id // ""')
            local stored_tree_hash=$(echo "$pr_data" | jq -r '.tree_hash // ""')
            local repo_root=$(echo "$pr_data" | jq -r '.repo_root')
            
            if [ -z "$jj_change_id" ]; then
                echo "PR #$pr_number has no JJ change ID, skipping push check"
                continue
            fi
            
            pushd "$repo_root" > /dev/null
            
            # Refresh merge commit info from GitHub
            local fresh_json=$(gh pr view "$pr_number" --json mergeCommit,baseRefName | jq -c)
            local fresh_merge_commit=$(echo "$fresh_json" | jq -r '.mergeCommit.oid // empty')
            local base_ref=$(echo "$fresh_json" | jq -r '.baseRefName // "master"')
            
            if [ -z "$fresh_merge_commit" ]; then
                echo "PR #$pr_number no longer has a merge commit, resetting merge_status"
                task "$pr_uuid" modify "merge_status:unstarted" "tree_hash:" "jj_change_id:"
                popd > /dev/null
                continue
            fi
            
            # Check if tree hash changed
            git fetch origin "$fresh_merge_commit" 2>/dev/null || true
            local fresh_tree_hash=$(git rev-parse "$fresh_merge_commit^{tree}" 2>/dev/null || echo "")
            
            if [ "$fresh_tree_hash" != "$stored_tree_hash" ]; then
                echo "⚠️  PR #$pr_number tree hash changed from $stored_tree_hash to $fresh_tree_hash"
                echo "   Resetting merge_status and updating dependencies"
                
                # Remove old merge commit dependency
                local old_merge_uuids=$(task "project:local-ci.$PROJECT" "commit_id.any:" "+MERGE_COMMIT" export | jq -r --arg tree "$stored_tree_hash" '.[] | select(.description | contains($tree)) | .uuid')
                while IFS= read -r old_uuid; do
                    if [ -n "$old_uuid" ]; then
                        task "$pr_uuid" modify "depends:-$old_uuid"
                    fi
                done <<< "$old_merge_uuids"
                
                # Create new merge commit task
                local new_merge_uuid=$(tw_upsert "project:local-ci.$PROJECT" "commit_id:$fresh_merge_commit" -- "repo_root:$repo_root" "description:Merge commit $fresh_merge_commit for PR #$pr_number")
                task "$new_merge_uuid" modify +MERGE_COMMIT
                task "$pr_uuid" modify "depends:$new_merge_uuid"
                
                # Update PR with new tree hash and reset status
                local fresh_jj_change_id=""
                if command -v jj >/dev/null 2>&1; then
                    fresh_jj_change_id=$(jj log -r "$fresh_merge_commit" --no-graph -T 'change_id' 2>/dev/null || echo "")
                fi
                
                task "$pr_uuid" modify "merge_status:unstarted" "tree_hash:$fresh_tree_hash" "jj_change_id:$fresh_jj_change_id"
                popd > /dev/null
                continue
            fi
            
            # Check if JJ change has GPG signature
            if command -v jj >/dev/null 2>&1; then
                local has_signature=$(jj log -r "$jj_change_id" --no-graph -T 'if(signature, "true", "false")' 2>/dev/null || echo "false")
                
                if [ "$has_signature" = "true" ]; then
                    echo "PR #$pr_number has GPG signature, pushing to $base_ref"
                    
                    if git push origin "$fresh_merge_commit:$base_ref"; then
                        echo "Successfully pushed PR #$pr_number to $base_ref"
                        task "$pr_uuid" modify "merge_status:pushed"
                    else
                        echo "Failed to push PR #$pr_number to $base_ref"
                    fi
                else
                    echo "PR #$pr_number JJ change $jj_change_id does not have GPG signature yet"
                fi
            else
                echo "jj command not available, cannot check GPG signature for PR #$pr_number"
            fi
            
            popd > /dev/null
        fi
    done <<< "$needsig_pr_uuids"
}
