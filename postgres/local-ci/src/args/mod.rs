// SPDX-License-Identifier: GPL-3.0-or-later

mod lexer;

use core::fmt;
use lexer::{ArgToken, lexed_args};
use lcilib::db::EntityType;
use std::process;
use std::sync::OnceLock;

static PROGRAM_NAME: OnceLock<String> = OnceLock::new();

/// A stack ID, displayed as `s` followed by a zero-padded 6-digit number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StackId(pub u32);

impl fmt::Display for StackId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "s{:06}", self.0)
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Info,
    Next,
    Refresh,
    Review,
    Run,
    Log,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Info => f.write_str("info"),
            Self::Next => f.write_str("next"),
            Self::Refresh => f.write_str("refresh"),
            Self::Review => f.write_str("review"),
            Self::Run => f.write_str("run"),
            Self::Log => f.write_str("log"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Pr(usize),
    Commit(String),
    Stack(StackId),
    None,
}

/// Options specific to the `log` action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogOptions {
    /// Filter logs to this entity type. `None` means all entity types.
    pub entity_type: Option<EntityType>,
    /// Only show logs at or after this datetime (YYYY-MM-DD or YYYY-MM-DD HH:MM:SS).
    pub since: Option<String>,
    /// Only show logs at or before this datetime (YYYY-MM-DD or YYYY-MM-DD HH:MM:SS).
    pub until: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArguments {
    pub action: Action,
    pub target: Target,
    /// Populated when `action == Action::Log`.
    pub log_options: Option<LogOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParseError {
    MultipleActions(Action, Action),
    MultipleTargetTypes(&'static str, &'static str),
    MultipleTargets(String, String),
    InvalidPrNumber(String),
    InvalidStackId(String),
    InvalidDatetime(String),
    MissingTarget(&'static str),
    MultipleLogEntityTypes(String, String),
    MultipleSince(String, String),
    MultipleUntil(String, String),
    LogOptionOnNonLogAction(&'static str),
    MissingAction,
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
            Self::InvalidStackId(s) => {
                write!(
                    f,
                    "Invalid stack ID: '{}'. Stack IDs must be 's' followed by exactly 6 digits (e.g. s000123).",
                    s
                )
            }
            Self::InvalidDatetime(s) => {
                write!(
                    f,
                    "Invalid datetime: '{}'. Expected format: YYYY-MM-DD or YYYY-MM-DD HH:MM:SS.",
                    s
                )
            }
            Self::MissingTarget(target_type) => {
                write!(
                    f,
                    "Target type '{}' specified but no target provided.",
                    target_type
                )
            }
            Self::MultipleLogEntityTypes(first, second) => {
                write!(
                    f,
                    "Multiple log entity types provided: '{}' and '{}'. Please specify only one.",
                    first, second
                )
            }
            Self::MultipleSince(first, second) => {
                write!(
                    f,
                    "Multiple --since values provided: '{}' and '{}'. Please specify only one.",
                    first, second
                )
            }
            Self::MultipleUntil(first, second) => {
                write!(
                    f,
                    "Multiple --until values provided: '{}' and '{}'. Please specify only one.",
                    first, second
                )
            }
            Self::LogOptionOnNonLogAction(opt) => {
                write!(
                    f,
                    "'{}' is only valid with the 'log' action.",
                    opt
                )
            }
            Self::MissingAction => {
                write!(f, "No action specified. Please provide an action (info, log, next, refresh, review, or run).")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Validate a datetime string: accepts YYYY-MM-DD or YYYY-MM-DD HH:MM:SS.
fn validate_datetime(s: &str) -> bool {
    // lmao at this vibe-coded function
    // YYYY-MM-DD
    if s.len() == 10 {
        let b = s.as_bytes();
        return b[4] == b'-' && b[7] == b'-'
            && b[..4].iter().all(u8::is_ascii_digit)
            && b[5..7].iter().all(u8::is_ascii_digit)
            && b[8..10].iter().all(u8::is_ascii_digit);
    }
    // YYYY-MM-DD HH:MM:SS
    if s.len() == 19 {
        let b = s.as_bytes();
        return b[4] == b'-' && b[7] == b'-' && b[10] == b' '
            && b[13] == b':' && b[16] == b':'
            && b[..4].iter().all(u8::is_ascii_digit)
            && b[5..7].iter().all(u8::is_ascii_digit)
            && b[8..10].iter().all(u8::is_ascii_digit)
            && b[11..13].iter().all(u8::is_ascii_digit)
            && b[14..16].iter().all(u8::is_ascii_digit)
            && b[17..19].iter().all(u8::is_ascii_digit);
    }
    false
}

#[allow(clippy::too_many_lines)]
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

    fn multiple_log_entries_error(existing: Option<EntityType>, new: Option<EntityType>) -> ParseError {
        ParseError::MultipleLogEntityTypes(
            existing.as_ref().map_or("all".to_owned(), EntityType::to_string),
            new.as_ref().map_or("all".to_owned(), EntityType::to_string),
        )
    }

    let mut action: Option<Action> = None;
    let mut target_type: Option<&'static str> = None;
    let mut target: Option<String> = None;

    // Log-specific options
    let mut log_entity_type: Option<Option<EntityType>> = None;
    let mut log_since: Option<String> = None;
    let mut log_until: Option<String> = None;

    // Track whether any log-specific tokens were seen, so we can error if
    // the action turns out not to be Log.
    let mut saw_log_option: Option<&'static str> = None;

    for token in lexed_args() {
        use ParseError as PE;
        match token {
            ArgToken::ProgramName(s) => {
                PROGRAM_NAME.set(s).ok();
            }

            // Actions
            ArgToken::Info => set_once(&mut action, Action::Info, PE::MultipleActions)?,
            ArgToken::Next => set_once(&mut action, Action::Next, PE::MultipleActions)?,
            ArgToken::Refresh => set_once(&mut action, Action::Refresh, PE::MultipleActions)?,
            ArgToken::Review => set_once(&mut action, Action::Review, PE::MultipleActions)?,
            ArgToken::Run => set_once(&mut action, Action::Run, PE::MultipleActions)?,
            ArgToken::Log => set_once(&mut action, Action::Log, PE::MultipleActions)?,

            // Target types
            ArgToken::Pr => {
                set_once(&mut target_type, "pr", PE::MultipleTargetTypes)?;
                set_once(&mut log_entity_type, Some(EntityType::PullRequest), multiple_log_entries_error)?;
            },
            ArgToken::Commit => {
                set_once(&mut target_type, "commit", PE::MultipleTargetTypes)?;
                set_once(&mut log_entity_type, Some(EntityType::Commit), multiple_log_entries_error)?;
            }
            ArgToken::Stack => {
                set_once(&mut target_type, "stack", PE::MultipleTargetTypes)?;
                set_once(&mut log_entity_type, Some(EntityType::Stack), multiple_log_entries_error)?;
            }

            // Log entity type filters (only valid with `log` action)
            ArgToken::Ack => {
                saw_log_option.get_or_insert("ack");
                set_once(&mut log_entity_type, Some(EntityType::Ack), multiple_log_entries_error)?;
            }
            ArgToken::System => {
                saw_log_option.get_or_insert("system");
                set_once(&mut log_entity_type, Some(EntityType::System), multiple_log_entries_error)?;
            }
            ArgToken::All => {
                saw_log_option.get_or_insert("all");
                set_once(&mut log_entity_type, None, multiple_log_entries_error)?;
            }

            // --since / --until
            ArgToken::Since(s) => {
                saw_log_option.get_or_insert("--since");
                if !validate_datetime(&s) {
                    return Err(ParseError::InvalidDatetime(s));
                }
                set_once(&mut log_since, s, ParseError::MultipleSince)?;
            }
            ArgToken::Until(s) => {
                saw_log_option.get_or_insert("--until");
                if !validate_datetime(&s) {
                    return Err(ParseError::InvalidDatetime(s));
                }
                set_once(&mut log_until, s, ParseError::MultipleUntil)?;
            }

            // Targets
            ArgToken::PrNumber(num) => {
                if target_type.is_none() {
                    target_type = Some("pr");
                }
                set_once(&mut target, num.to_string(), ParseError::MultipleTargets)?;
            }
            ArgToken::StackId(num) => {
                if target_type.is_none() {
                    target_type = Some("stack");
                }
                set_once(
                    &mut target,
                    format!("s{:06}", num),
                    ParseError::MultipleTargets,
                )?;
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
                // Could be a log entity type keyword used as a target type filter
                // for the log action (e.g. `log commit`, `log pr`, `log stack`).
                // Those are handled above via their own tokens. Anything reaching
                // here is a genuine git ref / jj change ID.
                if target_type.is_none() {
                    target_type = Some("commit");
                }
                set_once(&mut target, s, ParseError::MultipleTargets)?;
            }
        }
    }

    let action = action.ok_or(ParseError::MissingAction)?;

    // Validate that log-specific options are only used with the log action.
    if action != Action::Log && let Some(opt) = saw_log_option {
        return Err(ParseError::LogOptionOnNonLogAction(opt));
    }

    let final_target = match (target_type, target) {
        (None, None) => Target::None,
        (Some(x), None) => return Err(ParseError::MissingTarget(x)),

        (Some("pr"), Some(s)) => match s.parse() {
            Ok(num) => Target::Pr(num),
            Err(_) => return Err(ParseError::InvalidPrNumber(s)),
        },
        (Some("commit"), Some(s)) => Target::Commit(s),
        (Some("stack"), Some(s)) => {
            // s is already in the form s000123; parse the numeric part.
            match s[1..].parse::<u32>() {
                Ok(num) => Target::Stack(StackId(num)),
                Err(_) => return Err(ParseError::InvalidStackId(s)),
            }
        }
        (None, Some(_)) => unreachable!("target without inferred target type"),
        (Some(_), _) => unreachable!("invalid target type"),
    };

    let log_options = if action == Action::Log {
        Some(LogOptions {
            entity_type: log_entity_type.unwrap_or(None),
            since: log_since,
            until: log_until,
        })
    } else {
        None
    };

    Ok(CliArguments {
        action,
        target: final_target,
        log_options,
    })
}

pub fn usage() {
    let name = PROGRAM_NAME.get().map_or("local-ci", |s| s.as_str());
    eprintln!("Usage: {} [ACTION] [TARGET_TYPE] [TARGET] [LOG_OPTIONS]", name);
    eprintln!();
    eprintln!("Actions:");
    eprintln!("  info       Show information");
    eprintln!("  log        Show recent logs");
    eprintln!("  next       Show next item to action");
    eprintln!("  refresh    Refresh data");
    eprintln!("  review     Review a PR");
    eprintln!("  run        Run tests");
    eprintln!();
    eprintln!("Target Types:");
    eprintln!("  pr         Target is a pull request");
    eprintln!("  commit     Target is a commit or reference");
    eprintln!("  stack      Target is a stack");
    eprintln!();
    eprintln!("Targets:");
    eprintln!("  <number>       PR number (1-6 digits)");
    eprintln!("  merge-<number> Merge commit reference");
    eprintln!("  s<6digits>     Stack ID (e.g. s000123)");
    eprintln!("  <ref>          Git reference or jj change ID");
    eprintln!();
    eprintln!("Log Options (only valid with 'log' action):");
    eprintln!("  commit|pr|stack|ack|system|all   Filter by entity type (default: all)");
    eprintln!("  --since <datetime>               Show logs at or after this time");
    eprintln!("  --until <datetime>               Show logs at or before this time");
    eprintln!("  Datetime format: YYYY-MM-DD or YYYY-MM-DD HH:MM:SS");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {name} info 123");
    eprintln!("  {name} review pr 456");
    eprintln!("  {name} run commit merge-789");
    eprintln!("  {name} info stack s000042");
    eprintln!("  {name} log pr 123");
    eprintln!("  {name} log commit --since 2024-01-01");
    eprintln!("  {name} log all --since '2024-01-01 08:00:00' --until '2024-01-31 23:59:59'");
    eprintln!("  {name} log system");
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
