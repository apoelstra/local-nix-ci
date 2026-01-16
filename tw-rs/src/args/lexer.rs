// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;

/// A command-line argument.
#[derive(Debug)]
pub enum ArgToken {
    /// The 0th argument, which is typically the string used to invoke the program.
    ProgramName(String),
    /// A PR number. Any string of 1-6 (inclusive) numeric digits will be
    /// lexed as a PR number.
    PrNumber(usize),
    /// A reference to the merge commit of a PR. The string `merge-` followed
    /// by a `PrNumber` will be lexed as this.
    MergeRef(usize),
    /// The literal `info`. (Never interpreted as a number or ref.)
    Info,
    /// The literal `task-edit`. (Never interpreted as a number or ref.)
    TaskEdit,
    /// The literal `task-info`. (Never interpreted as a number or ref.)
    TaskInfo,
    /// The literal `refresh`. (Never interpreted as a number or ref.)
    Refresh,
    /// The literal `review`. (Never interpreted as a number or ref.)
    Review,
    /// The literal `run`. (Never interpreted as a number or ref.)
    Run,
    /// The literal `commit`. (Never interpreted as a number or ref.)
    Commit,
    /// The literal `pr`. (Never interpreted as a number or ref.)
    Pr,
    /// Anything else is assumed to be a git ref or jj change ID.
    MaybeRef(String),
}

fn parse_as_pr_number(s: &str) -> Option<usize> {
    if !s.is_empty() && s.len() <= 6 && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

/// Yields the string representing the program's invocation as well as
/// an interator over all other arguments.
pub fn lexed_args() -> impl Iterator<Item = ArgToken> {
    let mut is_first = true;
    env::args().map(move |s_arg| {
        if is_first { is_first = false; return ArgToken::ProgramName(s_arg); }

        match s_arg.as_str() {
            "info" => ArgToken::Info,
            "task-edit" => ArgToken::TaskEdit,
            "task-info" => ArgToken::TaskInfo,
            "refresh" => ArgToken::Refresh,
            "review" => ArgToken::Review,
            "run" => ArgToken::Run,
            "commit" => ArgToken::Commit,
            "pr" => ArgToken::Pr,
            _ => {
                if s_arg.len() > 6 && s_arg.as_bytes()[..6] == *b"merge-" {
                    if let Some(res) = parse_as_pr_number(&s_arg[6..]) {
                        return ArgToken::MergeRef(res);
                    }
                } else if let Some(res) = parse_as_pr_number(&s_arg) {
                    return ArgToken::PrNumber(res);
                }

                ArgToken::MaybeRef(s_arg)
            }
        }
    })

}