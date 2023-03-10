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
2. Then, ensure that you have a directory *one level above* the git repo that
   you want to run CI on. This directory should be available for repo-related
   files. See "Note on my `git worktree` setup" below for justification for this.
3. Similarly, ensure you have your git setup to fetch the `/head` and `/merge`
   refs from Github, and put them under `pr/`. To do this for all repos, add
   this your `~/.gitconfig`:
```
[remote "origin"]
        fetch = +refs/pull/*:refs/remotes/pr/*
        fetch = +refs/merge-requests/*:refs/remotes/pr/*
```
   The first line is for GIthub, the second for Gitlab.

4. Link all the shell files into `..`, and the appropriate `check-pr.nix` file:
```
ln -s "$PATH_TO_THIS_REPO/*.sh" ..
ln -s "$PATH_TO_THIS_REPO/org.project.check-pr.nix" ../check-pr.nix
```
5. For Rust projects, generate some `Cargo.lock` files:
```
cargo +nightly update -Z minimal-versions && cp Cargo.lock ../Cargo.minimal.lock
cargo +nightly update && cp Cargo.lock ../Cargo.latest.lock
```
   (You may need to do some `cargo update -p` iterating to fix the minimal lockfile,
   because projects don't test their minimal lockfiles.)
6. Generate the repo metadata: `../generate-repo-json.sh`

You should be good to go! Test it with
```
../test-pr.sh HEAD
```
which will test the single commit pointed to by `HEAD`.

In general, the syntax is
```
./test-pr.sh <ref | pr number> ["ACK"]
```
The script will attempt to interpret the first argument as a git ref. Failing
that, it will assume it is a PR number, and using the `pr/#/head` and `pr/#/merge`
refs from Github, figure out all the commits in the PR, and run itself on each
of those.

If the second argument is the string `ACK`, on success, it will use the `gh`
tool to ACK the PR. You may want to try running this command yourself before
letting the script do it. `gh` will initially ask some setup questions.

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

I am then typically cd'd into ``~/code/BlockstreamResearch/Simplicity/master/`.
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

