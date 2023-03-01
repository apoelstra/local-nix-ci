#!/usr/bin/env sh

URL=$(git remote get-url origin)
PULL_URL=$(echo "$URL" | sed 's/.git$/\/pull\//')

~/code/rsgit/master/target/release/label-pr "pr:master:$PULL_URL"

