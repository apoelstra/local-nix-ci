# Github PR Review and Testing Tool

This is a command-line tool for managing my PR review queue. It (ab)uses
[TaskWarrior](https://taskwarrior.org/) as a data storage backend to keep
track of projects, pull requests and commits.

Essentially the way it works is:

* Every commit gets a Taskwarrior task associated with it, which tracks
  both the review status (has the user read the code) and the CI status
  (has the system run tests on the code).
* Each PR gets a Taskwarrior task associated with it. This depends on the
  task for each of its commits, as well as the task for its merge commit.
* Each PR's "merge commit" is constructed with a commit message inspired
  by the [bitcoin-maintainer-tools github-merge.py](https://github.com/bitcoin-core/bitcoin-maintainer-tools/blob/main/github-merge.py)
  script; we do not trust the merge commits provided by Github, though we
  warn if our merges differ from theirs.

The typical workflow for a PR is:

1. The user first tells the system about the PR by invoking `local-ci pr <number>`
   or just `local-ci <number>`. This can be re-invoked to see the status of the PR.
   It shows all relevant commits and the overall review status.
2. The user reviews each commit with `local-ci commit [approve/nack] <ref>`, where
   again the word `commit` is unneeded, although if `ref` is all numeric and six
   or fewer characters, it will be interpreted as a PR number.
3. The user approves the PR with `local-ci pr approve <number>`, which will post a
   Github comment when all the CI jobs have passed.
4. The user approves the merge commit with `local-ci approve <ref>` where `<ref>`
   may be the special token `merge-<num>`. (If the merge is clean, then it will be
   automatically approved once all the other commits have been approved, and this
   step is unnecessary.)
5. Once all commits, including the merge, have been approved and passed tests,
   *and the user has signed the merge commit using the `check-and-sign.sh` script
   from this repo, then the merge will be pushed.

# License

While the Nix derivations and bash-based code is licensed as CC0 (public domain
dedication), this Rust tool is GPL v3. Please act accordingly.
