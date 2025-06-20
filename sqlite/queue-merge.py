#!/usr/bin/env python3
# Copyright (c) 2016-2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.

# This script will locally construct a merge commit for a pull request on a
# github repository, inspect it, sign it and optionally push it.

# The following temporary branches are created/overwritten and deleted:
# * pull/$PULL/base (the current master we're merging onto)
# * pull/$PULL/head (the current state of the remote pull request)
# * pull/$PULL/merge (github's merge)
# * pull/$PULL/local-merge (our merge)

# In case of a clean merge that is accepted by the user, the local branch with
# name $BRANCH is overwritten with the merged result, and optionally pushed.
import io
import os
from sys import stdin,stdout,stderr
import argparse
import re
import hashlib
import subprocess
import sys
import json
import codecs
import unicodedata
from urllib.request import Request, urlopen
from urllib.error import HTTPError

# External tools (can be overridden using environment)
GIT = os.getenv('GIT','git')
DOT_GIT_DIR = os.getenv(
    'GIT',
    subprocess.check_output([GIT, 'rev-parse', '--path-format=absolute', '--git-common-dir']).strip().decode('utf-8')
)
JJ = os.getenv('JJ', '/home/apoelstra/code/jj-vcs/jj/main/target/release/jj')
JJ_COMMIT_TPL = 'label("id", commit_id) ++ " " ++ description.first_line() ++ "\n"'
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


def get_symlink_files(rev='@'):
    """
    Return a sorted list of *symlink* paths that exist in <rev>.
    Pure-JJ version of the old Git ls-tree helper – no working-copy snapshot.
    """
    # Ask JJ for:  "<type> <path>\0" for every entry in the tree
    # file_type() →  "file" | "symlink" | "tree" | … :contentReference[oaicite:0]{index=0}
    output = subprocess.check_output(
        [
            JJ, '--ignore-working-copy', 'file', 'list',
            '-r', rev, '--no-pager',
            '-T', 'file_type ++ " " ++ path ++ "\\0"'
        ]
    )

    symlinks = []
    for entry in output.split(b'\0'):
        if not entry:
            continue
        ftype, path = entry.split(b' ', 1)
        if ftype == b'symlink':
            symlinks.append(path.decode())

    return sorted(symlinks)

def tree_sha512sum(rev='@'):
    """
    Deterministic aggregate SHA-512 of all files in <rev>, using the new
    `jj file show -s <sep>` batch mode.  Never touches the working copy.
    Format matches GNU coreutils’ `sha512sum` roll-up.
    """

    BUF_SZ = 256 * 1024
    base = [JJ, '--ignore-working-copy']

    # ── 1. stable, sorted list of repo-paths ─────────────────────────────
    paths = subprocess.check_output(
        base + ['file', 'list', '-r', rev,
                '--no-pager', '-T', 'path ++ "\\0"']
    ).split(b'\0')
    paths = sorted(p.decode() for p in paths if p)
    if not paths:
        return hashlib.sha512(b'').hexdigest()

    # ── 2. commit ID → unique delimiter string ──────────────────────────
    commit_id = subprocess.check_output(
        base + ['log', '-r', rev, '--no-graph', '-T', 'commit_id']
    ).rstrip()                                # e.g. b'9f6c1e…'
    delim = commit_id                        # guaranteed not to occur in blobs

    # ── 3. stream *all* blobs through one jj file show -------------------
    show = subprocess.Popen(
        base + ['file', 'show', '-r', rev, '-s', commit_id.decode(), '--'] + paths,
        stdout=subprocess.PIPE, text=False, bufsize=0
    )
    buf = bytearray()
    rdr = io.BufferedReader(show.stdout, buffer_size=BUF_SZ)

    overall = hashlib.sha512()

    def next_chunk():
        """Read more data; return False on EOF."""
        chunk = rdr.read(BUF_SZ)
        if chunk:
            buf.extend(chunk)
            return True
        return False

    for path_index, path in enumerate(paths):
        intern = hashlib.sha512()

        # Accumulate bytes until we find the delimiter *or* hit real EOF
        while True:
            i = buf.find(delim)
            if i != -1:
                # got a full file
                intern.update(buf[:i])
                del buf[:i + len(delim)]
                break
            # no delimiter yet
            if not next_chunk():
                if path_index != len(paths) - 1:
                    raise IOError("EOF before all files were read")
                # last file: whole remainder is its content
                intern.update(buf)
                buf.clear()
                break

        # update the aggregate hash in coreutils format
        overall.update(intern.hexdigest().encode('ascii'))
        overall.update(b'  ' + path.encode() + b'\n')

    rdr.close()
    show.wait()
    if show.returncode:
        raise IOError("`jj file show` failed with exit code %d" % show.returncode)

    return overall.hexdigest()

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

# Returns True if there are warnings, and False if everythinf is good.
def print_merge_details(pull_reference, title, branch, local_merge_change_id, acks, message):
    has_warnings = False

    print('{}{}{} {} {}into {}{}'.format(ATTR_RESET+ATTR_PR,pull_reference,ATTR_RESET,title,ATTR_RESET+ATTR_PR,branch,ATTR_RESET))
    subprocess.check_call([
        JJ, '--ignore-working-copy', 'show', '--no-pager', '-r', local_merge_change_id, '-s',
    ])
    # TODO check if there is any diff between the github `merge_commit` variable for the caller, and this commit.
    if acks is None:
        print('{}Top commit has no ACKs!{}'.format(ATTR_WARN, ATTR_RESET))
        has_warnings = True
    show_message = False
    if message is not None and '@' in message:
        print('{}Merge message contains an @!{}'.format(ATTR_WARN, ATTR_RESET))
        show_message = True
        has_warnings = True
    if message is not None and '<!-' in message:
        print('{}Merge message contains an html comment!{}'.format(ATTR_WARN, ATTR_RESET))
        show_message = True
        has_warnings = True
    if show_message:
        # highlight what might have tripped a warning
        message = message.replace('@', ATTR_HL + '@' + ATTR_RESET)
        message = message.replace('<!-', ATTR_HL + '<!-' + ATTR_RESET)
        print('-' * 75)
        print(message)
        print('-' * 75)
    return has_warnings

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
    signingkey = git_config_get('user.signingkey')
    if repo is None:
        print("ERROR: No repository configured. Use this command to set:", file=stderr)
        print("git config githubmerge.repository <owner>/<repo>", file=stderr)
        sys.exit(1)
    if signingkey is None:
        print("ERROR: No GPG signing key set. Set one using:",file=stderr)
        print("git config --global user.signingkey <key>",file=stderr)
        sys.exit(1)

    # Extract settings from command line
    args = parse_arguments()
    repo_from = args.repo_from or repo
    is_other_fetch_repo = repo_from != repo
    pull = str(args.pull[0])

    if host.startswith(('https:','http:')):
        host_repo = host+"/"+repo+".git"
        host_repo_from = host+"/"+repo_from+".git"
    else:
        host_repo = host+":"+repo
        host_repo_from = host+":"+repo_from

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
            print('ERROR: --repo-from is only supported for the master development branch')
            sys.exit(1)

    # Initialize source branches
    head_branch = f'pull/{pull}/head'
    base_branch = f'pull/{pull}/base'
    merge_branch = f'pull/{pull}/merge'
    local_merge_branch = f'pull/{pull}/local-merge'

    devnull = open(os.devnull, 'w', encoding="utf8")
    # Fetch the branches from Github. (We cannot do this with jj since it cannot
    # fetch arbitrary refs; see https://github.com/jj-vcs/jj/discussions/5388
    try:
        subprocess.check_call([GIT,'fetch','-q',host_repo_from,'+refs/pull/'+pull+'/*:refs/heads/pull/'+pull+'/*',
                                                          '+refs/heads/'+branch+':refs/heads/'+base_branch])
    except subprocess.CalledProcessError:
        print(f"ERROR: Cannot find pull request {pull_reference} or branch {branch} on {host_repo_from}.", file=stderr)
        sys.exit(3)

    # Validate that head ref exists on Github
    try:
        head_commit = subprocess.check_output([
            JJ, 'log', '--no-pager', '--no-graph',
            '-n', '1',
            '-T', 'commit_id',
            '-r', head_branch
        ]).decode('utf-8')
        assert len(head_commit) == 40
    except subprocess.CalledProcessError:
        print(f"ERROR: Cannot find head of pull request {pull_reference} on {host_repo_from}.", file=stderr)
        sys.exit(3)

    # Validate that merge ref exists on Github
    try:
        merge_commit = subprocess.check_output([
            JJ, 'log', '--no-pager', '--no-graph',
            '-n', '1',
            '-T', 'commit_id',
            '-r', merge_branch
        ]).decode('utf-8')
        assert len(merge_commit) == 40
    except subprocess.CalledProcessError:
        print(f"ERROR: Cannot find merge of pull request {pull_reference} on {host_repo_from}.", file=stderr)
        sys.exit(3)

    # Validate that base ref exists on Github
    try:
        base_commit = subprocess.check_output([
            JJ, 'log', '--no-pager', '--no-graph',
            '-n', '1',
            '-T', 'commit_id',
            '-r', base_branch
        ]).decode('utf-8')
        assert len(base_commit) == 40
    except subprocess.CalledProcessError:
        print(f"ERROR: Cannot find base of pull request {pull_reference} on {host_repo_from}.", file=stderr)
        sys.exit(3)

    # Create unsigned, no-description, merge commit.
    subprocess.check_call([
        JJ, '--config', 'signing.behavior=drop', '--ignore-working-copy',
        'new', '--no-edit',
        '-r', f'all:{head_commit} | {base_commit}'
    ])
    # (Racily) obtain the change ID of the commit we just made. It appears that `jj new`
    # cannot be made to just output the commit or change ID that it just created in a
    # machine-readable format. ChatGPT suggests parsing the human-readable output but
    # that seems likely to break, so instead I'm just gonna do the racy thing. Ultimately
    # this doesn't matter: we expect the resulting commit to be empty every time, and we
    # ask the user to sign this empty commit out-of-band, and so if we get the "wrong"
    # one then the worst case we'll have vandalized the description of an empty change.
    #
    # Notice that when obtaining the merge/base/head commits we used git commit IDs, but
    # now we are using jj change IDs. By using the change ID we ensure we keep pointed
    # at the "same" commit even after changing the description and having the user sign
    # it. But ofc this allows some malicious software to simultaneously edit the change
    # in some malicious way. To avoid this, we keep an eye on the tree hash and bail out
    # if it ever changes. (Within this script, we track a sha512-based tree hash, but
    # in local-ci.sh this isn't readily available so we just use the tree hash. If somebody
    # sha1-collides us, this will be visible after-the-fact at least since we stick the
    # sha512-based hash into the signed commit message.)
    local_merge_change_id = subprocess.check_output([
        JJ, 'log', '--no-pager', '--no-graph',
        '-n', '1',
        '-T', 'change_id',
        '-r', f'latest({head_commit}+ & {base_commit}+)'
    ]).decode('utf-8')
    assert len(local_merge_change_id) == 32

    local_merge_commit_id = subprocess.check_output([
        JJ, 'log', '--no-pager', '--no-graph',
        '-n', '1',
        '-T', 'commit_id',
        '-r', f'{local_merge_change_id}'
    ]).decode('utf-8')
    assert len(local_merge_commit_id) == 40

    local_merge_tree_hash = subprocess.check_output([
        GIT, 'log', '-1', local_merge_commit_id, '--pretty=%T'
    ]).strip().decode('utf-8')
    assert len(local_merge_commit_id) == 40

    # Check for conflicts
    try:
        subprocess.check_call([
            JJ, 'log', '--quiet', '-r' f'{local_merge_change_id} & ~conflicts()'
        ])
    except subprocess.CalledProcessError:
        print(f"ERROR: Cannot be merged cleanly. Conflicted change ID: {local_merge_change_id}",file=stderr)
        sys.exit(4)

    # Check for symlinks
    symlink_files = get_symlink_files(local_merge_change_id)
    for f in symlink_files:
        print(f"ERROR: File '{f}' was a symlink")
    if len(symlink_files) > 0:
        sys.exit(4)

    # Add a description
    # Description: title
    if title:
        message = f'Merge {pull_reference}: {title}\n\n'
    else:
        message = f'Merge {pull_reference}\n\n'
    # Description: commit list
    message += subprocess.check_output([
        JJ, '--ignore-working-copy',
        'log', '--no-graph', '--no-pager',
        '-r', f'{base_commit}..{head_commit}',
        '-T', JJ_COMMIT_TPL,
    ]).decode('utf-8')
    message += '\n\nPull request description:\n\n  ' + body.replace('\n', '\n  ') + '\n'

    # Description: comments and ACKs
    comments = retrieve_pr_comments(repo_from,pull,ghtoken) + retrieve_pr_reviews(repo_from,pull,ghtoken)
    if comments is None:
        print("ERROR: Could not fetch PR comments and reviews",file=stderr)
        sys.exit(1)
    acks = get_acks_from_comments(head_commit=head_commit, comments=comments)
    message += make_acks_message(head_commit=head_commit, acks=acks)

    # Description: tree SHA512
    try:
        first_sha512 = tree_sha512sum(local_merge_change_id)
    except subprocess.CalledProcessError:
        print("ERROR: Unable to compute tree hash")
        sys.exit(4)
    message += '\n\nTree-SHA512: ' + first_sha512

    subprocess.check_call([
        JJ, '--config', 'signing.behavior=drop', '--ignore-working-copy',
        'describe', '--no-edit',
        '-r', local_merge_change_id,
        '-m', message
    ])

    has_warnings = print_merge_details(pull_reference, title, branch, local_merge_change_id, acks, message)
    print()

    # If there are warnings, require explicit confirmation
    if has_warnings:
        confirmation = ask_prompt("Warnings detected. Type OK in all caps to continue, anything else to abort:")
        if confirmation != "OK":
            subprocess.check_call([JJ, 'abandon', '-r', local_merge_change_id])
            print(f"Merge {local_merge_change_id} abandoned. Bailing out.")
            sys.exit(1)

    # Insert into merge_pushes table
    try:
        repo_id = subprocess.check_output([
            'sqlite3', os.path.expanduser('~/local-ci.db'),
            f"SELECT id FROM repos WHERE dot_git_path = '{DOT_GIT_DIR}';"
        ]).decode('utf-8').strip()

        subprocess.check_call([
            'sqlite3', os.path.expanduser('~/local-ci.db'),
            f"INSERT INTO merge_pushes (repo_id, jj_change_id, tree_hash, target_branch) VALUES ({repo_id}, '{local_merge_change_id}', '{local_merge_tree_hash}', '{branch}');"
        ])
    except subprocess.CalledProcessError as e:
        print(f"Error inserting into merge_pushes table: {e}")
        sys.exit(1)

    # Queue the commit for testing
    try:
        subprocess.check_call([
            'local-ci.sh', 'queue-commit', local_merge_commit_id
        ])
    except subprocess.CalledProcessError as e:
        print(f"Error queuing commit for testing: {e}")
        sys.exit(1)

    # Notify the user
    try:
        notification_message = f"Merge for {pull_reference} queued for testing.\nTree hash: {local_merge_tree_hash}\nLocal change ID: {local_merge_change_id}\nCommit ID: {local_merge_commit_id}\n\nPlease sign the commit with your GPG key before it can be pushed."
        subprocess.check_call([
            'send-text.sh', notification_message
        ])
    except subprocess.CalledProcessError as e:
        print(f"Error sending notification: {e}")
        # Non-fatal error, continue

    print(f"Merge queued for testing. Change ID: {local_merge_change_id}")
    print("Please sign the commit with your GPG key before it can be pushed.")

if __name__ == '__main__':
    main()
