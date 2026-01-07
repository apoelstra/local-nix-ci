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
            
            # Check if all commits in related PRs are now successful
            check_and_approve_prs "$next_commit_uuid"
        else
            echo "Build failed for commit $commit_id"
            task "$next_commit_uuid" modify "ci_status:failed"
        fi
        
        popd > /dev/null
        echo "Finished processing commit $commit_id"
        echo
    done
}
