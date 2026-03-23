// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::Utc;
use core::fmt;
use std::error::Error;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    last_run: Instant,
    duration: Duration,
}

pub struct RateLimitToken(bool);

impl RateLimiter {
    /// Creates a new rate-limit token that will only allow an action to be taken
    /// if it has been `duration` since the last time.
    pub fn new(duration: Duration) -> Self {
        let now = Instant::now();
        let last_run = now.checked_sub(duration).unwrap_or(now);
        Self { last_run, duration }
    }

    /// Yields a token that can be used to run closures, as long as it's been at least
    /// `duration` since the last time.
    pub fn token(&mut self) -> RateLimitToken {
        let now = Instant::now();
        if now - self.last_run > self.duration {
            self.last_run = now;
            RateLimitToken(true)
        } else {
            RateLimitToken(false)
        }
    }
}

impl RateLimitToken {
    /// Constructs a rate-limit token that won't limit.
    pub fn ok_to_run() -> Self {
        Self(true)
    }

    /// Runs a closure if it's been at least `duration` since the last run.
    pub fn run<T>(&mut self, closure: impl FnOnce() -> T) -> Option<T> {
        self.0.then(closure)
    }
}

/// A token used to sleep for an increasing amount of time after each log message.
///
/// Can be reset with [`Self::reset`].
pub struct BackoffSleepToken {
    duration: Duration,
}

impl BackoffSleepToken {
    /// Creates a new token with the default (5 second) duration.
    pub fn new() -> Self {
        Self {
            duration: Duration::from_secs(5),
        }
    }

    /// Resets the token to its initial duration.
    pub fn reset(&mut self) {
        self.duration = Duration::from_secs(5);
    }

    /// Sleeps for the token's duration, then increases the duration by 50%.
    ///
    /// Maxes out at 8 hours to avoid overflow and because it seems gratuitous
    /// to sleep longer.
    pub async fn sleep_then_increment(&mut self) {
        tokio::time::sleep(self.duration).await;
        if self.duration < Duration::from_hours(8) {
            self.duration = (3 * self.duration) / 2;
        }
    }
}

/// Log a message with timestamp prefix
pub fn info<D: fmt::Display>(message: D) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    eprintln!("[{}] [INFO] {}", timestamp, message);
}

fn eprint_error(error: &dyn Error) {
    eprintln!("    Error: {error}");
    if let Some(sub) = error.source() {
        eprintln!("Caused by: {sub}");
        let mut error = sub;
        while let Some(sub) = error.source() {
            eprintln!("         : {sub}");
            error = sub;
        }
    }
}

/// Log a message with timestamp prefix
pub fn warn<D: fmt::Display>(error: &dyn Error, message: D) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    eprintln!("[{}] [WARN] {}", timestamp, message);
    eprint_error(error);
}

/// Log a message with timestamp prefix, and sleep for some amount of time
/// afterward.
///
/// The amount of time to sleep increases linearly, starting from 5 seconds
/// and adding 10 each time, so in case of an error cascade it wall take
/// quadratic time to post all the errors.
///
/// Each sleep will be longer than the last, to prevent error cascades from filling
/// whatever log buffer the user has. Call [`reset_error_sleep`] when things are
/// going well to reset the backoff count.
pub async fn warn_backoff<D: fmt::Display>(token: &mut BackoffSleepToken, error: &(dyn Error + Send + Sync + 'static), message: D) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    eprintln!("[{}] [WARN] {}", timestamp, message);
    eprint_error(error);
    eprintln!(
        "[{}] Sleeping for {} seconds...",
        timestamp,
        token.duration.as_secs()
    );
    token.sleep_then_increment().await;
}
