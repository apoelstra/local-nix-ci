// SPDX-License-Identifier: GPL-3.0-or-later

mod lexer;

use core::fmt;
use lexer::{ArgToken, lexed_args};
use std::process;
use std::sync::OnceLock;

static PROGRAM_NAME: OnceLock<String> = OnceLock::new();

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Info,
    Next,
    Refresh,
    Review,
    Run,
    TaskEdit,
    TaskInfo,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Info => f.write_str("info"),
            Self::Next => f.write_str("next"),
            Self::Refresh => f.write_str("refresh"),
            Self::Review => f.write_str("review"),
            Self::Run => f.write_str("run"),
            Self::TaskEdit => f.write_str("task-edit"),
            Self::TaskInfo => f.write_str("task-info"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Pr(usize),
    Commit(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArguments {
    pub action: Action,
    pub target: Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParseError {
    MultipleActions(Action, Action),
    MultipleTargetTypes(&'static str, &'static str),
    MultipleTargets(String, String),
    InvalidPrNumber(String),
    MissingTarget(&'static str),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultipleActions(first, second) => {
                write!(
                    f,
                    "Multiple actions provided: '{}' and '{}'. If '{}' is meant to be a git reference, try 'refs/heads/{}'.",
                    first, second, second, second
                )
            }
            Self::MultipleTargetTypes(first, second) => {
                write!(
                    f,
                    "Multiple target types provided: '{}' and '{}'. If '{}' is meant to be a git reference, try 'refs/heads/{}'.",
                    first, second, second, second
                )
            }
            Self::MultipleTargets(first, second) => {
                write!(
                    f,
                    "Multiple targets provided ('{}' and '{}'). Please specify only one target.",
                    first, second
                )
            }
            Self::InvalidPrNumber(s) => {
                write!(f, "Invalid PR number: '{}'. PR numbers must be numeric.", s)
            }
            Self::MissingTarget(target_type) => {
                write!(
                    f,
                    "Target type '{}' specified but no target provided.",
                    target_type
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

fn parse_args() -> Result<CliArguments, ParseError> {
    fn set_once<T>(
        existing: &mut Option<T>,
        new: T,
        error: fn(T, T) -> ParseError,
    ) -> Result<(), ParseError> {
        if let Some(existing) = existing.take() {
            return Err(error(existing, new));
        }
        *existing = Some(new);
        Ok(())
    }

    let mut action = None;
    let mut target_type = None;
    let mut target = None;

    for token in lexed_args() {
        match token {
            ArgToken::ProgramName(s) => {
                PROGRAM_NAME.set(s).ok(); // Ignore error if already set
                continue;
            }
            ArgToken::Info => set_once(&mut action, Action::Info, ParseError::MultipleActions)?,
            ArgToken::Next => set_once(&mut action, Action::Next, ParseError::MultipleActions)?,
            ArgToken::Refresh => {
                set_once(&mut action, Action::Refresh, ParseError::MultipleActions)?
            }
            ArgToken::Review => set_once(&mut action, Action::Review, ParseError::MultipleActions)?,
            ArgToken::Run => set_once(&mut action, Action::Run, ParseError::MultipleActions)?,
            ArgToken::TaskEdit => {
                set_once(&mut action, Action::TaskEdit, ParseError::MultipleActions)?
            }
            ArgToken::TaskInfo => {
                set_once(&mut action, Action::TaskInfo, ParseError::MultipleActions)?
            }

            ArgToken::Pr => set_once(&mut target_type, "pr", ParseError::MultipleTargetTypes)?,
            ArgToken::Commit => {
                set_once(&mut target_type, "commit", ParseError::MultipleTargetTypes)?
            }

            ArgToken::PrNumber(num) => {
                if target_type.is_none() {
                    target_type = Some("pr");
                }
                set_once(&mut target, num.to_string(), ParseError::MultipleTargets)?;
            }
            ArgToken::MergeRef(num) => {
                if target_type.is_none() {
                    target_type = Some("commit");
                }
                set_once(
                    &mut target,
                    format!("merge-{num}"),
                    ParseError::MultipleTargets,
                )?;
            }
            ArgToken::MaybeRef(s) => {
                if target_type.is_none() {
                    target_type = Some("commit");
                }
                set_once(&mut target, s, ParseError::MultipleTargets)?;
            }
        }
    }

    let action = action.unwrap_or(Action::Info);

    let final_target = match (target_type, target) {
        (None, None) => Target::None,
        (Some(x), None) => return Err(ParseError::MissingTarget(x)),

        (Some("pr"), Some(s)) => match s.parse() {
            Ok(num) => Target::Pr(num),
            Err(_) => return Err(ParseError::InvalidPrNumber(s)),
        },
        (Some("commit"), Some(s)) => Target::Commit(s),
        (None, Some(_)) => unreachable!("target without inferred target type"),
        (Some(_), _) => unreachable!("invalid target type"),
    };

    Ok(CliArguments {
        action,
        target: final_target,
    })
}

pub fn usage() {
    let name = PROGRAM_NAME.get().map(|s| s.as_str()).unwrap_or("tw-rs");
    eprintln!("Usage: {} [ACTION] [TARGET_TYPE] [TARGET]", name);
    eprintln!();
    eprintln!("Actions:");
    eprintln!("  approve    Approve a PR");
    eprintln!("  info       Show information (default)");
    eprintln!("  nack       Reject a PR");
    eprintln!("  refresh    Refresh data");
    eprintln!("  review     Review a PR");
    eprintln!("  run        Run tests");
    eprintln!();
    eprintln!("Target Types:");
    eprintln!("  pr         Target is a pull request");
    eprintln!("  commit     Target is a commit or reference");
    eprintln!();
    eprintln!("Targets:");
    eprintln!("  <number>       PR number (1-6 digits)");
    eprintln!("  merge-<number> Merge commit reference");
    eprintln!("  <ref>          Git reference or jj change ID");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  tw-rs info 123");
    eprintln!("  tw-rs review pr 456");
    eprintln!("  tw-rs run commit merge-789");
    eprintln!("  tw-rs approve commit main");
}

pub fn parse_cli() -> CliArguments {
    match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("Error: {}", error);
            eprintln!();
            usage();
            process::exit(1);
        }
    }
}
