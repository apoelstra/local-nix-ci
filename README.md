# Nix Tooling for local CI

This is a set of nix expressions that I use to check every commit of various PRs
to the projects that I do code review on. This tree is immature and in-flux and
it is **not** recomended for general use.

Furthermore, it assumes that Nix is able to use a lot of CPU time. In particular,
for rust-bitcoin, empirically it spends 20+ minutes per commit running tests,
even when remote-building on a 32-core machine. Other projects are generally
much less bad than this, but it's unlikely that you'll want to run any of them
on an unplugged laptop, say.

Having said that, if you are interested in using this, read on.

# Setup

1. Clone this repo somewhere.
2. Run `cargo build --release` in the `postgres/` directory and symlink it into
   your `~/bin/` (or maybe you can run `cargo install`? I don't know what this
   does.)
3. Run `local-ci run` in a tmux session.

## Review and Testing

To review a PR, go to your local checkout and:

1. Fetch it from Github or Forgejo: `local-ci refresh <number>`
2. Run `local-ci <number>` to see a summary, then run `local-ci next <number>`
   repeatedly to review each commit and finally ACK the PR.
3. Wait for the CI system to test each commit, create a merge commit, put it in
   a "stack", and test that. (To see available stacks and their test status just
   run `local-ci info`.)
4. Sign the commits in the stack (see below)

## Signing and Pushing

TODO implement this properly:

1. For each commit in the stack you want to push, run

```
sqlite/check-and-sign.sh <pr number> <jj change ID of the merge>
```

2. Then run `local-ci info` and refresh a few times til the CI system has noticed
   your signatures and updated the git commit IDs.
3. Push: `git push origin <tip of stack>:master`
4. Manually refresh all the PRs you pushed: `local-ci <number> refresh` so that the
   system doesn't try to re-do them on a new stack. (It downloads all changes from
   Github in real-time but somehow isn't notified about merges? I dunno. TODO
   investigate why you have to do this manually)

# Contributing

Contributions are welcome. I will try to keep the public version of this repo
up to date. Probably before we can do any real collaborative development, I
(or you :)) needs to write a test .nix file which attempts to build the latest
commit of every repo, so we can make sure that changes to `andrew-utils.nix`
don't break other stuff.

# Note on my `git worktree` setup

The way I have all my git repos setup for all my projects is to create a directory
for it, e.g. `~/code/BlockstreaResearch/Simplicity`, then to clone the directory
into `master/` (or whatever the default branch is called) inside that.

I am then typically cd'd into `~/code/BlockstreamResearch/Simplicity/master/`.
To create PRs I add new worktrees parallel to `master`, like `git worktree add
../2023-03--example-pr`. I also have a dedicated `pr-review` worktree which is
usually checked-out to random detached-HEAD commits etc when I'm investigating
PRs.

Throughought I will use `..` to refer to "the directory above the repo", in this
case `~/code/BlockstreamResearch/Simplicity/`. In a typical repo, `..` contains
some one-off scripts related to the repo, maybe some test inputs that I need
frequently, and a `README` with some personal notes about when/how I set things
up, weirdness related to build, etc. I may also have a `shell.nix` and `default.nix`
here in case I need Nix to build stuff but the repo itself is not Nix-enabled.

# License

The code in the `crate2nix/` directory is licensed under MIT and Apache by the
crate2nix developers. (In general, changes to these files should be proposed
upstream at https://github.com/kolloch before/in parallel to being added here.
We want to minimize divergence.)

The code in the top-level directory is licensed CC0 by me, Andrew Poelstra. All
contributions to these files will also be CC0 licensed.

