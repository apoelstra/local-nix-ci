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
    /// A stack ID. The string `s` followed by exactly 6 numeric digits
    /// (e.g. `s000123`) will be lexed as a stack ID.
    StackId(u32),
    /// A reference to the merge commit of a PR. The string `merge-` followed
    /// by a `PrNumber` will be lexed as this.
    MergeRef(usize),
    /// The literal `info`. (Never interpreted as a number or ref.)
    Info,
    /// The literal `next`. (Never interpreted as a number or ref.)
    Next,
    /// The literal `refresh`. (Never interpreted as a number or ref.)
    Refresh,
    /// The literal `review`. (Never interpreted as a number or ref.)
    Review,
    /// The literal `run`. (Never interpreted as a number or ref.)
    Run,
    /// The literal `log`. (Never interpreted as a number or ref.)
    Log,
    /// The literal `commit`. (Never interpreted as a number or ref.)
    Commit,
    /// The literal `pr`. (Never interpreted as a number or ref.)
    Pr,
    /// The literal `stack`. (Never interpreted as a number or ref.)
    Stack,
    /// The literal `ack`. (Used as a log filter entity type.)
    Ack,
    /// The literal `system`. (Used as a log filter entity type.)
    System,
    /// The literal `all`. (Used as a log filter entity type.)
    All,
    /// `--since` followed by a datetime value (YYYY-MM-DD or YYYY-MM-DD HH:MM:SS).
    Since(String),
    /// `--until` followed by a datetime value (YYYY-MM-DD or YYYY-MM-DD HH:MM:SS).
    Until(String),
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

/// Returns true if the string is a valid stack ID (s followed by exactly 6 digits).
fn parse_as_stack_id(s: &str) -> Option<u32> {
    if s.len() == 7
        && s.as_bytes()[0] == b's'
        && s.as_bytes()[1..].iter().all(u8::is_ascii_digit)
    {
        s[1..].parse().ok()
    } else {
        None
    }
}

/// Yields the string representing the program's invocation as well as
/// an iterator over all other arguments.
pub fn lexed_args() -> impl Iterator<Item = ArgToken> {
    let mut args = env::args();
    let mut pending_flag: Option<&'static str> = None;
    let mut is_first = true;

    std::iter::from_fn(move || {
        let s_arg = args.next()?;

        if is_first {
            is_first = false;
            return Some(ArgToken::ProgramName(s_arg));
        }

        // If we're expecting a value for a flag, consume it now.
        if let Some(flag) = pending_flag.take() {
            return Some(match flag {
                "--since" => ArgToken::Since(s_arg),
                "--until" => ArgToken::Until(s_arg),
                _ => unreachable!("unexpected pending flag"),
            });
        }

        match s_arg.as_str() {
            "info" => Some(ArgToken::Info),
            "next" => Some(ArgToken::Next),
            "refresh" => Some(ArgToken::Refresh),
            "review" => Some(ArgToken::Review),
            "run" => Some(ArgToken::Run),
            "log" => Some(ArgToken::Log),
            "commit" => Some(ArgToken::Commit),
            "pr" => Some(ArgToken::Pr),
            "stack" => Some(ArgToken::Stack),
            "ack" => Some(ArgToken::Ack),
            "system" => Some(ArgToken::System),
            "all" => Some(ArgToken::All),
            "--since" => {
                pending_flag = Some("--since");
                // Recurse by calling the closure again via the iterator machinery.
                // We do this by returning a sentinel and letting the next call handle it.
                // Instead, we pull the next arg immediately.
                let value = args.next().unwrap_or_default();
                Some(ArgToken::Since(value))
            }
            "--until" => {
                let value = args.next().unwrap_or_default();
                Some(ArgToken::Until(value))
            }
            _ => {
                if s_arg.len() > 6 && s_arg.as_bytes()[..6] == *b"merge-" {
                    if let Some(res) = parse_as_pr_number(&s_arg[6..]) {
                        return Some(ArgToken::MergeRef(res));
                    }
                } else if let Some(res) = parse_as_stack_id(&s_arg) {
                    return Some(ArgToken::StackId(res));
                } else if let Some(res) = parse_as_pr_number(&s_arg) {
                    return Some(ArgToken::PrNumber(res));
                }

                Some(ArgToken::MaybeRef(s_arg))
            }
        }
    })
}
