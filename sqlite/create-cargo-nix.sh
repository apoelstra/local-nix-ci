#!/usr/bin/env bash

set -euo pipefail
shopt -s nullglob

unset GIT_DIR

command -v git >/dev/null 2>&1 || { echo "git is required but not installed. Aborting."; exit 1; }
command -v crate2nix >/dev/null 2>&1 || { echo "crate2nix is required but not installed. Aborting."; exit 1; }

usage() {
  echo "Usage: $0 <repo path> <commit> <lockfile>" >&2
  echo >&2
  echo "Here <lockfile> may be an absolute path or a path relative to <repo path>." >&2
  echo >&2
  echo "Outputs a pathname on stdout to a Cargo.nix file created and placed in the nix store." >&2
  exit 1
}

if [ -z "${1:-}" ] || [ -z "${2:-}" ] || [ -z "${3:-}" ]; then
  usage
fi

REPO_PATH=$1
COMMIT_ID=$2
LOCKFILE=$3

# Check that REPO_PATH is valid.
if [ ! -d "$REPO_PATH" ]; then
  echo "Repo path must be a directory; got $REPO_PATH"
  usage
fi
pushd "$REPO_PATH" > /dev/null

# Now that we're in the repo, check that COMMIT_ID is valid.
if [ "$(git cat-file -t "$COMMIT_ID" 2>/dev/null)" != "commit" ]; then
  echo "Commit ID $COMMIT_ID does not appear to be a commit." >&2
  usage
fi

# Finally, for LOCKFILE we have a few choices

# 1. It can be a (ref to a) blob in the repo.
if [ "$(git cat-file -t "$LOCKFILE" 2>/dev/null)" == "blob" ]; then
  TEMPDIR=$(mktemp -d)
  git show "$LOCKFILE" > "$TEMPDIR/Cargo.lock"
# 2. It can be a path (either relative to the repo dir, or absolute)
elif [ -f "$LOCKFILE" ]; then
  TEMPDIR=$(mktemp -d)
  cp "$LOCKFILE" "$TEMPDIR/Cargo.lock"
else
  echo "Lockfile does not appear to be a git blob or a path to an ordinary file." >&2
  echo "If it is a relative path, it must be with respect to the repo directory." >&2
  echo "Repo directory: $REPO_DIR" >&2
  echo "Purported lockfile: $LOCKFILE" >&2
  echo >&2
  usage
fi

# Ok, everything is validated. We have created a temporary directory and moved
# a lockfile in there. Next copy, all the Cargo.toml files. Also copy any lib.rs
# or main.rs files, because if the Cargo.toml doesn't specify any source files,
# cargo will use these as defaults, and cargo-metadata will get upset to find
# they don't exist.
git ls-tree -r --name-only "$COMMIT_ID" | grep -E "(Cargo.toml|lib.rs|main.rs)$" | while read -r fullname; do
  dirname=$(dirname "$fullname")
  mkdir -p "$TEMPDIR/$dirname"
  git show "$COMMIT_ID:$fullname" > "$TEMPDIR/$fullname"
done

# Now, in the temp directory, attempt to generate cargo.nix.
pushd "$TEMPDIR" > /dev/null
crate2nix generate >&2 || (
  echo "crate2nix generate failed; bailing out." >&2
  echo "Warning: leaving temp directory in place: $TEMPDIR" >&2
  exit 1
)
# Then add the file to the store. `nix-store --add` will output the name.
nix-store --add Cargo.nix

rm -rf -- "$TEMPDIR"

echo "Success." >&2
