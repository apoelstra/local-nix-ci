#!/usr/bin/env python3
# Copyright (c) 2016-2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.

# This script computes a detailed merge commit message, and does some sanity
# checks. If not given `-y` or `--yes`, it will prompt the user to OK the
# output if any sanity checks fail.
#
# Extracted from github-merge.py in the bitcoin-core/bitcoin-maintainer-tools
# Github repo, commit 12bc3a10a2abfbac2252557b7b682fbc56e459b4 but heavily
# modified.
#
# Requires the use of jj (and a jj-initialized git repo, which can be obtained
# non-destructively by running `jj git init --colocate` in any git repo),
# because we need to construct a merge commit without checking anything out,
# and git cannot do that.

# In case of a clean merge that is accepted by the user, the local branch with
# name $BRANCH is overwritten with the merged result, and optionally pushed.
import os
from sys import stdin,stdout,stderr
import argparse
import re
import subprocess
import sys
import json
import codecs
import unicodedata
from urllib.request import Request, urlopen
from urllib.error import HTTPError

from compute_sha512_root import commit_id_from_git_or_jj, tree_sha512sum

# External tools (can be overridden using environment)
GIT = os.getenv('GIT','git')
SHELL = os.getenv('SHELL','bash')

# OS specific configuration for terminal attributes
ATTR_RESET = ''
ATTR_PR = ''
ATTR_NAME = ''
ATTR_WARN = ''
ATTR_HL = ''
COMMIT_FORMAT = '%H %s (%an)%d'
if os.name == 'posix': # if posix, assume we can use basic terminal escapes
    ATTR_RESET = '\033[0m'
    ATTR_PR = '\033[1;36m'
    ATTR_NAME = '\033[0;36m'
    ATTR_WARN = '\033[1;31m'
    ATTR_HL = '\033[95m'
    COMMIT_FORMAT = '%C(bold blue)%H%Creset %s %C(cyan)(%an)%Creset%C(green)%d%Creset'

def sanitize(s, newlines=False):
    '''
    Strip control characters (optionally except for newlines) from a string.
    This prevent text data from doing potentially confusing or harmful things
    with ANSI formatting, linefeeds bells etc.
    '''
    return ''.join(ch for ch in s if unicodedata.category(ch)[0] != "C" or (ch == '\n' and newlines))

def git_config_get(option, default=None):
    '''
    Get named configuration option from git repository.
    '''
    try:
        return subprocess.check_output([GIT,'config','--get',option]).rstrip().decode('utf-8')
    except subprocess.CalledProcessError:
        return default

def get_response(req_url, ghtoken):
    req = Request(req_url)
    if ghtoken is not None:
        req.add_header('Authorization', 'token ' + ghtoken)
    return urlopen(req)

def sanitize_ghdata(rec):
    '''
    Sanitize comment/review record coming from github API in-place.
    This currently sanitizes the following:
    - ['title'] PR title (optional, may not have newlines)
    - ['body'] Comment body (required, may have newlines)
    It also checks rec['user']['login'] (required) to be a valid github username.

    When anything more is used, update this function!
    '''
    if 'title' in rec: # only for PRs
        rec['title'] = sanitize(rec['title'], newlines=False)
    if rec['body'] is None:
        rec['body'] = ''
    rec['body'] = sanitize(rec['body'], newlines=True)

    if rec['user'] is None: # User deleted account
        rec['user'] = {'login': '[deleted]'}
    else:
        # "Github username may only contain alphanumeric characters or hyphens'.
        # Sometimes bot have a "[bot]" suffix in the login, so we also match for that
        # Use \Z instead of $ to not match final newline only end of string.
        if not re.match(r'[a-zA-Z0-9-]+(\[bot\])?\Z', rec['user']['login'], re.DOTALL):
            raise ValueError('Github username contains invalid characters: {}'.format(sanitize(rec['user']['login'])))
    return rec

def retrieve_json(req_url, ghtoken, use_pagination=False):
    '''
    Retrieve json from github.
    Return None if an error happens.
    '''
    try:
        reader = codecs.getreader('utf-8')
        if not use_pagination:
            return sanitize_ghdata(json.load(reader(get_response(req_url, ghtoken))))

        obj = []
        page_num = 1
        while True:
            req_url_page = '{}?page={}'.format(req_url, page_num)
            result = get_response(req_url_page, ghtoken)
            obj.extend(json.load(reader(result)))

            link = result.headers.get('link', None)
            if link is not None:
                link_next = [l for l in link.split(',') if 'rel="next"' in l]
                if len(link_next) > 0:
                    page_num = int(link_next[0][link_next[0].find("page=")+5:link_next[0].find(">")])
                    continue
            break
        return [sanitize_ghdata(d) for d in obj]
    except HTTPError as e:
        error_message = e.read()
        print('Warning: unable to retrieve pull information from github: %s' % e)
        print('Detailed error: %s' % error_message)
        return None
    except Exception as e:
        print('Warning: unable to retrieve pull information from github: %s' % e)
        return None

def retrieve_pr_info(repo,pull,ghtoken):
    req_url = "https://api.github.com/repos/"+repo+"/pulls/"+pull
    return retrieve_json(req_url,ghtoken)

def retrieve_pr_comments(repo,pull,ghtoken):
    req_url = "https://api.github.com/repos/"+repo+"/issues/"+pull+"/comments"
    return retrieve_json(req_url,ghtoken,use_pagination=True)

def retrieve_pr_reviews(repo,pull,ghtoken):
    req_url = "https://api.github.com/repos/"+repo+"/pulls/"+pull+"/reviews"
    return retrieve_json(req_url,ghtoken,use_pagination=True)

def ask_prompt(text):
    print(text,end=" ",file=stderr)
    stderr.flush()
    reply = stdin.readline().rstrip()
    print("",file=stderr)
    return reply

def get_symlink_files(commit = 'HEAD'):
    files = sorted(subprocess.check_output([GIT, 'ls-tree', '--full-tree', '-r', commit]).splitlines())
    ret = []
    for f in files:
        if (int(f.decode('utf-8').split(" ")[0], 8) & 0o170000) == 0o120000:
            ret.append(f.decode('utf-8').split("\t")[1])
    return ret

def get_acks_from_comments(head_commit, comments) -> dict:
    # Look for abbreviated commit id, because not everyone wants to type/paste
    # the whole thing and the chance of collisions within a PR is small enough
    head_abbrev = head_commit[0:6]
    acks = {}
    for c in comments:
        review = [
            l for l in c["body"].splitlines()
            if "ACK" in l
            and head_abbrev in l
            and not l.startswith("> ")  # omit if quoted comment
            and not l.startswith("    ")  # omit if markdown indentation
        ]
        if review:
            acks[c['user']['login']] = review[0]
    return acks

def make_acks_message(head_commit, acks) -> str:
    if acks:
        ack_str ='\n\nACKs for top commit:\n'.format(head_commit)
        for name, msg in acks.items():
            ack_str += '  {}:\n'.format(name)
            ack_str += '    {}\n'.format(msg)
    else:
        ack_str ='\n\nTop commit has no ACKs.\n'
    return ack_str

def print_merge_details(pull_reference, title, branch, base_branch, head_branch, acks, message):
    has_warnings = False

    print('{}{}{} {} {}into {}{}'.format(ATTR_RESET+ATTR_PR,pull_reference,ATTR_RESET,title,ATTR_RESET+ATTR_PR,branch,ATTR_RESET))
    subprocess.check_call([GIT,'--no-pager','log','--graph','--topo-order','--pretty=tformat:'+COMMIT_FORMAT,base_branch+'..'+head_branch])
    if acks is not None:
        if acks:
            print('{}ACKs:{}'.format(ATTR_PR, ATTR_RESET))
            for ack_name, ack_msg in acks.items():
                print('* {} {}({}){}'.format(ack_msg, ATTR_NAME, ack_name, ATTR_RESET))
        else:
            print('{}Top commit has no ACKs!{}'.format(ATTR_WARN, ATTR_RESET))
    show_message = False
    if message is not None and '@' in message:
        print('{}Merge message contains an @!{}'.format(ATTR_WARN, ATTR_RESET))
        show_message = True
    if message is not None and '<!-' in message:
        print('{}Merge message contains an html comment!{}'.format(ATTR_WARN, ATTR_RESET))
        show_message = True
    if show_message:
        # highlight what might have tripped a warning
        message = message.replace('@', ATTR_HL + '@' + ATTR_RESET)
        message = message.replace('<!-', ATTR_HL + '<!-' + ATTR_RESET)
        print('-' * 75)
        print(message)
        print('-' * 75)

def parse_arguments():
    epilog = '''
        In addition, you can set the following git configuration variables:
        githubmerge.repository (mandatory, e.g. <owner>/<repo>),
        githubmerge.pushmirrors (default: none, comma-separated list of mirrors to push merges of the master development branch to, e.g. `git@gitlab.com:<owner>/<repo>.git,git@github.com:<owner>/<repo>.git`),
        user.signingkey (mandatory),
        user.ghtoken (default: none).
        githubmerge.merge-author-email (default: Email from git config),
        githubmerge.host (default: git@github.com),
        githubmerge.branch (no default),
        githubmerge.testcmd (default: none).
    '''
    parser = argparse.ArgumentParser(description='Utility to merge, sign and push github pull requests',
            epilog=epilog)
    parser.add_argument('--repo-from', '-r', metavar='repo_from', type=str, nargs='?',
        help='The repo to fetch the pull request from. Useful for monotree repositories. Can only be specified when branch==master. (default: githubmerge.repository setting)')
    parser.add_argument('--local-merge-ref', '-c', metavar='local_merge_ref', type=str, nargs='?', required=True,
        help='The git ref or jj change ID of the local merge to describe.')
    parser.add_argument('--yes', '-y', action='store_true',
        help='Noninteractive mode (do not prompt the user if warnings are present)')
    parser.add_argument('pull', metavar='PULL', type=int, nargs=1,
        help='Pull request ID to merge')
    parser.add_argument('branch', metavar='BRANCH', type=str, nargs='?',
        default=None, help='Branch to merge against (default: githubmerge.branch setting, or base branch for pull, or \'master\')')
    return parser.parse_args()

def main():
    # Extract settings from git repo
    repo = git_config_get('githubmerge.repository')
    host = git_config_get('githubmerge.host','git@github.com')
    opt_branch = git_config_get('githubmerge.branch',None)
    merge_author_email = git_config_get('githubmerge.merge-author-email',None)
    testcmd = git_config_get('githubmerge.testcmd')
    ghtoken = git_config_get('user.ghtoken')
    if repo is None:
        stderr.write("ERROR: No repository configured. Use this command to set:\n")
        stderr.write("git config githubmerge.repository <owner>/<repo>\n")
        sys.exit(1)

    # Extract settings from command line
    args = parse_arguments()
    repo_from = args.repo_from or repo
    is_other_fetch_repo = repo_from != repo
    pull = str(args.pull[0])

    if host.startswith(('https:','http:')):
        host_repo = f'{host}/{repo}.git'
        host_repo_from = f'{host}/{repo_from}.git'
    else:
        host_repo = f'{host}:{repo}'
        host_repo_from = f'{host}:{repo_from}'

    # Receive pull information from github
    info = retrieve_pr_info(repo_from,pull,ghtoken)
    if info is None:
        sys.exit(1)
    title = info['title'].strip()
    body = info['body'].strip()
    pull_reference = repo_from + '#' + pull
    # precedence order for destination branch argument:
    #   - command line argument
    #   - githubmerge.branch setting
    #   - base branch for pull (as retrieved from github)
    #   - 'master'
    branch = args.branch or opt_branch or info['base']['ref'] or 'master'

    if branch == 'master':
        push_mirrors = git_config_get('githubmerge.pushmirrors', default='').split(',')
        push_mirrors = [p for p in push_mirrors if p]  # Filter empty string
    else:
        push_mirrors = []
        if is_other_fetch_repo:
            stderr.write('ERROR: --repo-from is only supported for the master development branch\n')
            sys.exit(1)

    # Initialize source branches
    head_branch = f'pull/{pull}/head'
    base_branch = f'pull/{pull}/base'
    merge_branch = f'pull/{pull}/merge'
    local_merge_branch = args.local_merge_ref

    devnull = open(os.devnull, 'w', encoding="utf8")
    # Fetch the branches from Github. (We cannot do this with jj, even if we wanted to,
    # since it cannot fetch arbitrary refs; see https://github.com/jj-vcs/jj/discussions/5388
    try:
        subprocess.check_call([GIT,'fetch','-q',host_repo_from,'+refs/pull/'+pull+'/*:refs/heads/pull/'+pull+'/*',
                                                          '+refs/heads/'+branch+':refs/heads/'+base_branch])
    except subprocess.CalledProcessError:
        stderr.write(f"ERROR: Cannot find pull request {pull_reference} or branch {branch} on {host_repo_from}.\n")
        sys.exit(3)

    # Validate that expected refs exist in the repo
    commits = []
    for ref in [head_branch, base_branch, merge_branch, local_merge_branch]:
        try:
            commit = commit_id_from_git_or_jj(ref, exit_on_fail = False)
            if len(commit) != 40:
                stderr.write(f"ERROR: {ref} of pull request {pull_reference} on {host_repo_from} does not have a unique commit ID.\n")
                sys.exit(3)

            commits.append(commit)
        except subprocess.CalledProcessError:
            stderr.write(f"ERROR: Cannot find {ref} of pull request {pull_reference} on {host_repo_from}.\n")
            sys.exit(3)
    head_commit = commits[0]
    base_commit = commits[1]
    merge_commit = commits[2]
    local_merge_commit = commits[3]

    # Check for symlinks -- this one can't be overridden.
    symlink_files = get_symlink_files(local_merge_commit)
    for f in symlink_files:
        stderr.write(f"ERROR: File '{f}' was a symlink\n")
    if len(symlink_files) > 0:
        sys.exit(4)

    has_warnings = False
    # Check diff between local merge and Github merge 
    diff = subprocess.check_output([GIT, 'diff', f'{merge_commit}..{local_merge_commit}'])
    if diff:
        stderr.write("WARNING: merge differs from github!\n")
        stderr.write(diff.decode('utf-8'))
        stderr.write(f"\nGithub merge commit: {merge_commit}\n")
        stderr.write(f"   Our merge commit: {local_merge_commit}\n")
        stderr.write(f"Run git diff {merge_commit}..{local_merge_commit} to reproduce.\n\n")
        has_warnings = True

    # Compute description
    # Description: title
    if title:
        message = f'Merge {pull_reference}: {title}\n\n'
    else:
        message = f'Merge {pull_reference}\n\n'

    # Description: commit list
    message += subprocess.check_output([
        GIT, '--no-pager', 'log', '--no-merges', '--topo-order',
        '--pretty=format:%H %s (%an)', f'{base_commit}..{head_commit}'
    ]).decode('utf-8')
    # Description: PR body
    message += '\n\nPull request description:\n\n  ' + body.replace('\n', '\n  ') + '\n'

    # Description: comments and ACKs
    comments = retrieve_pr_comments(repo_from,pull,ghtoken) + retrieve_pr_reviews(repo_from,pull,ghtoken)
    if comments is None:
        stderr.write("ERROR: Could not fetch PR comments and reviews\n")
        sys.exit(1)

    acks = get_acks_from_comments(head_commit=head_commit, comments=comments)
    if not acks:
        stderr.write(f'{ATTR_WARN}Top commit has no ACKs!{ATTR_RESET}\n')
        has_warnings = True
    message += make_acks_message(head_commit=head_commit, acks=acks)

    # Description: tree SHA512
    try:
        first_sha512 = tree_sha512sum(local_merge_commit)
    except subprocess.CalledProcessError:
        stderr.write("ERROR: Unable to compute tree hash\n")
        sys.exit(4)
    message += '\n\nTree-SHA512: ' + first_sha512

    # Final message checks
    if '@' in message:
        stderr.write(f'{ATTR_WARN}Merge message contains an @!{ATTR_RESET}\n')
        has_warnings = True
    if '<!-' in message:
        has_warnings = True

    if has_warnings and not args.yes:
        stderr.write("At least one warning occurred.\n\nCommit message:\n")
        # highlight what might have tripped a warning
        disp_message = message.replace('@', ATTR_HL + '@' + ATTR_RESET)
        disp_message = disp_message.replace('<!-', ATTR_HL + '<!-' + ATTR_RESET)
        stderr.write('-' * 75)
        stderr.write('\n')
        stderr.write(disp_message)
        stderr.write('\n')
        stderr.write('-' * 75)
        stderr.write('\n')

        while True:
            reply = ask_prompt("Type 'OK' in all caps to continue, or anything else to bail out.")
            if reply == 'OK':
                break
            elif reply == '':
                pass
            else:
                stderr.write("Bailing out.\n")
                sys.exit(1)

    print(message)

if __name__ == '__main__':
    main()
