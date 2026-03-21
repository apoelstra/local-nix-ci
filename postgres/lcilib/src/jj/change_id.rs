// SPDX-License-Identifier: GPL-3.0-or-later

use core::{fmt, str};
use postgres_types::{FromSql, ToSql};
use std::ffi::OsStr;

/// A representation of a jj change ID.
///
/// When deserialized, validated to be 32 'k'-'z' digits, but stored
/// as a string to allow efficient use with xshell.
#[derive(Clone, Default, PartialEq, Eq, Debug, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct ChangeId(String);

impl str::FromStr for ChangeId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 32 {
            return Err(Error::WrongLength {
                expected: 32,
                got: s.len(),
            });
        }
        for ch in s.chars() {
            if !('k'..='z').contains(&ch) {
                return Err(Error::InvalidCharacter { ch });
            }
        }
        Ok(Self(s.to_owned()))
    }
}

impl fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<OsStr> for ChangeId {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl ChangeId {
    /// A string representation of the change ID
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// An 8-character jj prefix
    pub fn prefix8(&self) -> &str {
        &self.0[..8]
    }
}

#[derive(Debug)]
pub enum Error {
    WrongLength { expected: usize, got: usize },
    InvalidCharacter { ch: char },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::WrongLength { expected, got } => {
                write!(f, "jj change ID had length {} (expected {})", got, expected)
            }
            Self::InvalidCharacter { ch } => {
                write!(f, "invalid character '{}' in jj change ID", ch)
            }
        }
    }
}

impl std::error::Error for Error {}
