# Should be sourced from local-ci.sh.
# Not intended to be run directly.

run() {
    local backoff_sec=30
    local sleep_sec=$backoff_sec # trigger warning/status update on first iteration
    local filter=(project:local-ci claimedby: or "claimedby:$HOSTNAME")

    while true; do
        local busy=false
        local warnings=()

        # 1. First, instantiate anything that is uninstantiated.
        local unstarted
        while IFS= read -d $'\0' -r unstarted; do
            # FIXME because instantiation is mostly single-threaded, and in this case
            #  takes a very long time due to IFD, we should background these and only
            #  block if there are more than 4 of them (say) running.
            local project=$(echo "$unstarted" | jq -r .project)
            local git_dir=$(echo "$unstarted" | jq -r .repo_root)
            local commit_id=$(echo "$unstarted" | jq -r .commit_id)
            local pr_number=$(echo "$unstarted" | jq -r .pr_number) # may be null

            # Compute nixfile path
            local nixfile_path="$(
                echo "$project" | sed "s#^local-ci.#$LOCAL_CI_PATH/$LOCAL_CI_WORKTREE/#"
            )".check-pr.nix
            if ! [ -f "$nixfile_path" ]; then
                warnings+=("Skipped job with project $project since nixfile path is not found: $nixfile_path'")
                continue
            fi
            
            echo "Task: $project";

            nix-instantiate \
                --arg inlineJsonConfig "{ gitDir = $git_dir; projectName = \"$project\"; }" \
                --arg inlineCommitList "[ \"$commit_id\" ]" \
                --argstr prNum "$pr_number" \
                "$nixfile_path"

        done < <(
            task "${filter[@]}" ci_status:unstarted derivation: export | \
                jq --raw-output0 -c '.[]'
        )

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
                fi
                sleep_sec=1
            fi
        fi
    done
}
