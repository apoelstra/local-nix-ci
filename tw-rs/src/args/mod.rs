// SPDX-License-Identifier: GPL-3.0-or-later

mod lexer;

use std::process;
use lexer::{ArgToken, lexed_args};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Approve,
    Info,
    Nack,
    Refresh,
    Review,
    Run,
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
pub enum ParseError {
    MultipleActions(String, String),
    MultipleTargetTypes(String, String),
    MultipleTargets,
    MergeRefWithPrType(usize),
    InvalidPrNumber(String),
    MissingTarget(String),
    InvalidTargetForCommitType(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MultipleActions(first, second) => {
                write!(f, "Multiple actions provided: '{}' and '{}'. If '{}' is meant to be a git reference, try 'refs/heads/{}'.", first, second, second, second)
            }
            ParseError::MultipleTargetTypes(first, second) => {
                write!(f, "Multiple target types provided: '{}' and '{}'. If '{}' is meant to be a git reference, try 'refs/heads/{}'.", first, second, second, second)
            }
            ParseError::MultipleTargets => {
                write!(f, "Multiple targets provided. Please specify only one target.")
            }
            ParseError::MergeRefWithPrType(pr_num) => {
                write!(f, "Cannot use merge reference 'merge-{}' with target type 'pr'. Did you mean to use 'commit' instead of 'pr'?", pr_num)
            }
            ParseError::InvalidPrNumber(s) => {
                write!(f, "Invalid PR number: '{}'. PR numbers must be numeric.", s)
            }
            ParseError::MissingTarget(target_type) => {
                write!(f, "Target type '{}' specified but no target provided.", target_type)
            }
            ParseError::InvalidTargetForCommitType(s) => {
                write!(f, "Invalid target '{}' for commit type.", s)
            }
        }
    }
}

impl std::error::Error for ParseError {}

fn parse_args() -> Result<CliArguments, ParseError> {
    let mut action = None;
    let mut target_type = None;
    let mut target = None;
    
    for token in lexed_args().skip(1) { // Skip program name
        match token {
            ArgToken::ProgramName(_) => unreachable!("Should have been skipped"),
            
            ArgToken::Approve => {
                if let Some(existing) = action {
                    return Err(ParseError::MultipleActions(
                        format!("{:?}", existing).to_lowercase(),
                        "approve".to_string()
                    ));
                }
                action = Some(Action::Approve);
            }
            ArgToken::Info => {
                if let Some(existing) = action {
                    return Err(ParseError::MultipleActions(
                        format!("{:?}", existing).to_lowercase(),
                        "info".to_string()
                    ));
                }
                action = Some(Action::Info);
            }
            ArgToken::Nack => {
                if let Some(existing) = action {
                    return Err(ParseError::MultipleActions(
                        format!("{:?}", existing).to_lowercase(),
                        "nack".to_string()
                    ));
                }
                action = Some(Action::Nack);
            }
            ArgToken::Refresh => {
                if let Some(existing) = action {
                    return Err(ParseError::MultipleActions(
                        format!("{:?}", existing).to_lowercase(),
                        "refresh".to_string()
                    ));
                }
                action = Some(Action::Refresh);
            }
            ArgToken::Review => {
                if let Some(existing) = action {
                    return Err(ParseError::MultipleActions(
                        format!("{:?}", existing).to_lowercase(),
                        "review".to_string()
                    ));
                }
                action = Some(Action::Review);
            }
            ArgToken::Run => {
                if let Some(existing) = action {
                    return Err(ParseError::MultipleActions(
                        format!("{:?}", existing).to_lowercase(),
                        "run".to_string()
                    ));
                }
                action = Some(Action::Run);
            }
            
            ArgToken::Pr => {
                if target_type.is_some() {
                    return Err(ParseError::MultipleTargetTypes(
                        "pr".to_string(),
                        "pr".to_string() // This will be overwritten by the actual second type
                    ));
                }
                target_type = Some("pr");
            }
            ArgToken::Commit => {
                if let Some(existing) = target_type {
                    return Err(ParseError::MultipleTargetTypes(
                        existing.to_string(),
                        "commit".to_string()
                    ));
                }
                target_type = Some("commit");
            }
            
            ArgToken::PrNumber(num) => {
                if target.is_some() {
                    return Err(ParseError::MultipleTargets);
                }
                target = Some(("pr_number", num.to_string()));
            }
            ArgToken::MergeRef(num) => {
                if target.is_some() {
                    return Err(ParseError::MultipleTargets);
                }
                target = Some(("merge_ref", num.to_string()));
            }
            ArgToken::MaybeRef(s) => {
                if target.is_some() {
                    return Err(ParseError::MultipleTargets);
                }
                target = Some(("maybe_ref", s));
            }
        }
    }
    
    let action = action.unwrap_or(Action::Info);
    
    let final_target = match (target_type, target) {
        (None, None) => Target::None,
        (None, Some(("pr_number", num_str))) => {
            Target::Pr(num_str.parse().unwrap()) // This should always succeed since lexer validated it
        }
        (None, Some(("merge_ref", num_str))) => {
            Target::Commit(format!("merge-{}", num_str))
        }
        (None, Some(("maybe_ref", s))) => {
            Target::Commit(s)
        }
        (Some("pr"), None) => {
            return Err(ParseError::MissingTarget("pr".to_string()));
        }
        (Some("commit"), None) => {
            return Err(ParseError::MissingTarget("commit".to_string()));
        }
        (Some("pr"), Some(("pr_number", num_str))) => {
            Target::Pr(num_str.parse().unwrap())
        }
        (Some("pr"), Some(("merge_ref", num_str))) => {
            return Err(ParseError::MergeRefWithPrType(num_str.parse().unwrap()));
        }
        (Some("pr"), Some(("maybe_ref", s))) => {
            match s.parse::<usize>() {
                Ok(num) => Target::Pr(num),
                Err(_) => return Err(ParseError::InvalidPrNumber(s)),
            }
        }
        (Some("commit"), Some(("pr_number", num_str))) => {
            Target::Commit(num_str)
        }
        (Some("commit"), Some(("merge_ref", num_str))) => {
            Target::Commit(format!("merge-{}", num_str))
        }
        (Some("commit"), Some(("maybe_ref", s))) => {
            Target::Commit(s)
        }
        _ => unreachable!("Invalid target type"),
    };
    
    Ok(CliArguments {
        action,
        target: final_target,
    })
}

fn usage(error: ParseError) {
    eprintln!("Error: {}", error);
    eprintln!();
    eprintln!("Usage: tw-rs [ACTION] [TARGET_TYPE] [TARGET]");
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

pub fn parse_cli() {
    match parse_args() {
        Ok(_args) => {
            // For now, just succeed silently
            // Later this will be used by the main function
        }
        Err(error) => {
            usage(error);
            process::exit(1);
        }
    }
}
