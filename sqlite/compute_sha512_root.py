#!/usr/bin/env python3
# Copyright (c) 2016-2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.

# This script computes a SHA512-based hash of the contents of a git commit.
# Extracted from github-merge.py in the bitcoin-core/bitcoin-maintainer-tools
# Github repo, commit 12bc3a10a2abfbac2252557b7b682fbc56e459b4

import io, os, subprocess, sys
import argparse
import hashlib

# External tools (can be overridden using environment)
GIT = os.getenv('GIT', 'git')
JJ = os.getenv('JJ', '/home/apoelstra/code/jj-vcs/jj/main/target/release/jj')

def tree_sha512sum(commit='HEAD'):
    # request metadata for entire tree, recursively
    files = []
    blob_by_name = {}
    for line in subprocess.check_output([GIT, 'ls-tree', '--full-tree', '-r', commit]).splitlines():
        name_sep = line.index(b'\t')
        metadata = line[:name_sep].split() # perms, 'blob', blobid
        assert(metadata[1] == b'blob')
        name = line[name_sep+1:]
        files.append(name)
        blob_by_name[name] = metadata[2]

    files.sort()
    # open connection to git-cat-file in batch mode to request data for all blobs
    # this is much faster than launching it per file
    p = subprocess.Popen([GIT, 'cat-file', '--batch'], stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    overall = hashlib.sha512()
    for f in files:
        blob = blob_by_name[f]
        # request blob
        p.stdin.write(blob + b'\n')
        p.stdin.flush()
        # read header: blob, "blob", size
        reply = p.stdout.readline().split()
        assert(reply[0] == blob and reply[1] == b'blob')
        size = int(reply[2])
        # hash the blob data
        intern = hashlib.sha512()
        ptr = 0
        while ptr < size:
            bs = min(65536, size - ptr)
            piece = p.stdout.read(bs)
            if len(piece) == bs:
                intern.update(piece)
            else:
                raise IOError('Premature EOF reading git cat-file output')
            ptr += bs
        dig = intern.hexdigest()
        assert(p.stdout.read(1) == b'\n') # ignore LF that follows blob data
        # update overall hash with file hash
        overall.update(dig.encode("utf-8"))
        overall.update("  ".encode("utf-8"))
        overall.update(f)
        overall.update("\n".encode("utf-8"))
    p.stdin.close()
    if p.wait():
        raise IOError('Non-zero return value executing git cat-file')
    return overall.hexdigest()


# Takes a git commit ref or jj change ID in the current repo, and returns the
# commit ID.
#
# If `exit_on_fail` is true, if both git and jj fail then their stderrs are
# written to stderr and the script aborted. Otherwise we just raise a CalledProcessError
# from the `git rev-parse` invocation.
def commit_id_from_git_or_jj(ref, exit_on_fail = True):
    try:
        commit = subprocess.check_output([GIT, 'rev-parse', ref + '^{commit}'], stderr=subprocess.PIPE).decode('utf-8').strip()
    except subprocess.CalledProcessError as git_e:
        try:
            # Attempt jj if git fails; maybe the user provided a change ID>
            commit = subprocess.check_output([JJ, 'log', '--no-graph', '-r', ref, '-T', 'commit_id'], stderr=subprocess.PIPE).decode('utf-8').strip()
        except subprocess.CalledProcessError as jj_e:
            if exit_on_fail:
                sys.stderr.write("Both git and jj failed.\n\n")
                sys.stderr.write("git stderr:\n")
                sys.stderr.write(git_e.stderr.decode('utf8'))
                sys.stderr.write("\n")
                sys.stderr.write("jj stderr:\n")
                sys.stderr.write(jj_e.stderr.decode('utf8'))
                sys.stderr.write("\n")

                sys.exit(1)
            else:
                raise git_e

    return commit


def parse_arguments():
    parser = argparse.ArgumentParser(description='Utility to compute the SHA512-based tree hash of a commit.')
    parser.add_argument('ref', metavar='REF', type=str,
        default='HEAD', help='git ref or jj change ID to compute the tree hash of')
    return parser.parse_args()


def main():
    # Extract settings from command line
    args = parse_arguments()
    commit = commit_id_from_git_or_jj(args.ref)
    print(tree_sha512sum(commit))


if __name__ == '__main__':
    main()
