// SPDX-License-Identifier: GPL-3.0-or-later

//! Small helpers for reading interactive user input.
//!
//! All of these bail out on EOF (`read_line` returning zero bytes). The
//! previous ad-hoc loops re-prompted forever on EOF, so piping an empty input
//! into an interactive command (e.g. `local-ci review ... </dev/null`) spun
//! the process at 100% CPU printing menus.

use std::io::{self, BufRead, Write};

/// The outcome of reading a line of user input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    /// A line was read (without its trailing newline).
    Line(String),
    /// stdin hit EOF (or an I/O error occurred): no more input is available.
    Eof,
}

/// Reads one line from the reader. Returns [`ReadOutcome::Eof`] when the
/// reader is exhausted instead of looping or panicking.
pub fn read_line_from<R: BufRead>(reader: &mut R) -> ReadOutcome {
    let mut buf = String::new();
    match reader.read_line(&mut buf) {
        Ok(0) => ReadOutcome::Eof,
        Ok(_) => ReadOutcome::Line(buf.trim_end_matches(['\n', '\r']).to_owned()),
        Err(e) => {
            // Treat I/O errors as end-of-input: every caller wants to stop
            // prompting rather than spin, and the interactive commands are
            // best-effort UI.
            eprintln!("Error reading input: {e}");
            ReadOutcome::Eof
        }
    }
}

/// Reads one line from stdin.
pub fn read_line() -> ReadOutcome {
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    read_line_from(&mut lock)
}

/// Asks a yes/no question until it gets a definitive answer.
///
/// Returns `None` on EOF: there is no user left to answer, and callers should
/// abort the operation rather than guess.
pub fn ask_yes_no_from<R: BufRead>(
    reader: &mut R,
    mut out: impl Write,
    query: impl core::fmt::Display,
) -> Option<bool> {
    loop {
        write!(out, "{query} (y/n) ").ok()?;
        out.flush().ok()?;

        match read_line_from(reader) {
            ReadOutcome::Eof => {
                writeln!(out).ok()?;
                return None;
            }
            ReadOutcome::Line(choice) => match choice.trim().to_ascii_lowercase().as_str() {
                "y" | "yes" => return Some(true),
                "n" | "no" => return Some(false),
                _ => continue,
            },
        }
    }
}

/// Asks a yes/no question on stdin/stdout. Returns `None` on EOF.
pub fn ask_yes_no(query: impl core::fmt::Display) -> Option<bool> {
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    ask_yes_no_from(&mut lock, io::stdout(), query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_line() {
        let mut input = b"hello\n".as_slice();
        assert_eq!(read_line_from(&mut input), ReadOutcome::Line("hello".to_owned()));
    }

    #[test]
    fn strips_crlf() {
        let mut input = b"hello\r\n".as_slice();
        assert_eq!(read_line_from(&mut input), ReadOutcome::Line("hello".to_owned()));
    }

    /// Regression test: EOF used to produce an empty string over and over,
    /// which callers treated as "invalid choice, ask again" => infinite loop.
    #[test]
    fn eof_is_reported_not_looped() {
        let mut input = b"".as_slice();
        assert_eq!(read_line_from(&mut input), ReadOutcome::Eof);

        // EOF after some input.
        let mut input = b"one line\n".as_slice();
        assert_eq!(read_line_from(&mut input), ReadOutcome::Line("one line".to_owned()));
        assert_eq!(read_line_from(&mut input), ReadOutcome::Eof);
    }

    #[test]
    fn yes_no_accepts_various_answers() {
        for (input, expected) in [
            (&b"y\n"[..], Some(true)),
            (&b"Y\n"[..], Some(true)),
            (&b"yes\n"[..], Some(true)),
            (&b"YES\n"[..], Some(true)),
            (&b"n\n"[..], Some(false)),
            (&b"No\n"[..], Some(false)),
            (&b"  n  \n"[..], Some(false)),
        ] {
            let mut input = input;
            let mut out = Vec::new();
            assert_eq!(
                ask_yes_no_from(&mut input, &mut out, "Continue?"),
                expected,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn yes_no_reprompts_on_garbage() {
        let mut input = b"maybe\nblah\ny\n".as_slice();
        let mut out = Vec::new();
        assert_eq!(
            ask_yes_no_from(&mut input, &mut out, "Continue?"),
            Some(true)
        );
        // The prompt is written once per attempt.
        assert_eq!(out.iter().filter(|&&b| b == b'?').count(), 3);
    }

    /// Regression test: EOF used to spin the prompt forever. It must return
    /// None so callers can abort.
    #[test]
    fn yes_no_returns_none_on_eof() {
        let mut input = b"".as_slice();
        let mut out = Vec::new();
        assert_eq!(ask_yes_no_from(&mut input, &mut out, "Continue?"), None);

        // Garbage then EOF: reprompt happened, then EOF aborts.
        let mut input = b"junk\n".as_slice();
        let mut out = Vec::new();
        assert_eq!(ask_yes_no_from(&mut input, &mut out, "Continue?"), None);
    }
}
