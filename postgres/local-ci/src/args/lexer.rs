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
    /// A stack ID. The string `s` followed by at least 1 numeric digit
    /// (e.g. `s123` or `s000123`) will be lexed as a stack ID.
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
    /// The literal `reset`. (Never interpreted as a number or ref.)
    Reset,
    /// The literal `run`. (Never interpreted as a number or ref.)
    Run,
    /// The literal `log`. (Never interpreted as a number or ref.)
    Log,
    /// The literal `help`. (Never interpreted as a number or ref.)
    Help,
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
    /// The literal "--no-color"
    NoColorFlag,
}

fn parse_as_pr_number(s: &str) -> Option<usize> {
    if !s.is_empty() && s.len() <= 6 && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

/// Returns true if the string is a valid stack ID (s followed by at least 1 digit).
fn parse_as_stack_id(s: &str) -> Option<u32> {
    let digits = s.strip_prefix('s')?;
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        digits.parse().ok()
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
            "reset" => Some(ArgToken::Reset),
            "run" => Some(ArgToken::Run),
            "log" => Some(ArgToken::Log),
            "help" => Some(ArgToken::Help),
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
            "--no-color" => Some(ArgToken::NoColorFlag),
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

#[cfg(test)]
mod tests {
    use super::*;

    // docstring: "The string `s` followed by at least 1 numeric digit (e.g. `s123` or
    // `s000123`) will be lexed as a stack ID."
    //
    // Regression: the implementation requires len > 2 (s + >=2 digits), so `s1`
    // and other single-digit IDs silently fall through to `MaybeRef` and are
    // then treated as git refs, producing a confusing "commit not found" error.
    #[test]
    fn stack_id_accepts_single_digit_per_docstring() {
        assert_eq!(parse_as_stack_id("s1"), Some(1));
        assert_eq!(parse_as_stack_id("s9"), Some(9));
    }

    #[test]
    fn stack_id_accepts_various_lengths() {
        // Examples from docstrings.
        assert_eq!(parse_as_stack_id("s123"), Some(123));
        assert_eq!(parse_as_stack_id("s000123"), Some(123));
        // Not a stack ID: no digits.
        assert_eq!(parse_as_stack_id("s"), None);
        assert_eq!(parse_as_stack_id("se"), None);
        // Not a stack ID: leading char not 's'.
        assert_eq!(parse_as_stack_id("x123"), None);
    }

    #[test]
    fn pr_number_accepts_one_to_six_digits() {
        assert_eq!(parse_as_pr_number("1"), Some(1));
        assert_eq!(parse_as_pr_number("123456"), Some(123456));
        assert_eq!(parse_as_pr_number(""), None);
        assert_eq!(parse_as_pr_number("1234567"), None);
        assert_eq!(parse_as_pr_number("12a"), None);
    }

    // Regression: `local-ci info stack <garbage>` used to panic in
    // args::mod because the stack-ID branch sliced `s[1..]` without
    // checking char boundaries. This test just exercises parse_as_stack_id
    // on multi-byte input to document that it does not panic; the
    // corresponding fix lives in mod.rs.
    #[test]
    fn stack_id_rejects_non_ascii_without_panicking() {
        assert_eq!(parse_as_stack_id("sé1"), None);
        assert_eq!(parse_as_stack_id("é123"), None);
        assert_eq!(parse_as_stack_id("\u{1f980}"), None);
    }
}
