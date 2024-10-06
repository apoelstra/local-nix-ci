#!/bin/bash

## Draft written by ChatGPT on 2024-09-24.
## Modified by ASP to fix shellcheck comments, to change the global
## `queue_file` to be uppercase, and to correct a couple paths.

set -e

# File to store the queue
QUEUE_FILE="$HOME/local-ci-queue.json"

# Function to add commands to the queue
add_command() {
    local cmd="$1"
    local dir="$PWD"
    # Check if queue file exists and is not empty
    if [[ -s "$QUEUE_FILE" ]]; then
        # Read current contents and add new command along with the directory
        jq --arg cmd "$cmd" --arg dir "$dir" '.queue += [{"cmd": $cmd, "dir": $dir}]' "$QUEUE_FILE" > "$QUEUE_FILE.tmp" && mv "$QUEUE_FILE.tmp" "$QUEUE_FILE"
    else
        # Create new queue with the command and directory
        echo '{"queue": [{"cmd": "'"$cmd"'", "dir": "'"$dir"'"}]}' > "$QUEUE_FILE"
    fi
}

# Function to run commands from the queue
run_commands() {
    while true; do
        if [[ -s "$QUEUE_FILE" ]]; then
            # Extract the first command and directory, and update the queue
            local first_item
            local cmd
            local dir
	    first_item=$(jq '.queue[0]' "$QUEUE_FILE")
	    cmd=$(echo "$first_item" | jq -r '.cmd')
	    dir=$(echo "$first_item" | jq -r '.dir')
            jq '.queue |= .[1:]' "$QUEUE_FILE" > "$QUEUE_FILE.tmp" && mv "$QUEUE_FILE.tmp" "$QUEUE_FILE"

            # Check if the command is not null or empty
            if [[ "$cmd" != "null" ]] && [[ -n "$cmd" ]]; then
                # Change to the directory before executing the command
                pushd "$dir" > /dev/null
                send-text.sh "Starting: $cmd"
                if eval "$cmd"; then
                    send-text.sh "Success: $cmd"
		else
                    send-text.sh "FAILURE: $cmd"
                fi
                popd > /dev/null
            fi
        else
            # No commands left, break the loop
            break
        fi
        # Check again for new commands after one execution
    done
}

# Main logic based on the command line argument
case "$1" in
    add)
        shift
        add_command "$*"
        ;;
    run)
        run_commands
        ;;
    *)
        echo "Usage: $0 {add <cmd> | run}"
        exit 1
        ;;
esac
