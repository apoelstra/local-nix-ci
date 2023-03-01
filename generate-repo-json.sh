#!/usr/bin/env bash

echo "Outputting to ../repo.json."

GITDIR=$(git rev-parse --path-format=absolute --git-common-dir)
DATADIR=$(realpath "$GITDIR/../../")
REPONAME=$(basename "$DATADIR")

cat > ../repo.json << EOJ
{
	"gitUrl": "$(git remote get-url origin)",
	"gitDir": "$GITDIR",
	"dataDir": "$DATADIR",
	"repoName": "$REPONAME",
	"lockFiles": [
$(
	shopt -s nullglob
	FIRST=true
	for file in "$DATADIR"/*lock; do
		# fuck json
		if [ "$FIRST" != "true" ]; then
			echo ","
		fi
		echo -n "		\"$file\"";
		FIRST=false
	done
)
	]
}
EOJ

