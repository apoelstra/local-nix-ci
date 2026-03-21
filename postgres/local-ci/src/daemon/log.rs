// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::Utc;
use core::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static ERROR_BACKOFF: AtomicU64 = AtomicU64::new(5);

pub fn reset_error_sleep() {
    ERROR_BACKOFF.store(5, Ordering::Relaxed);
}

/// Log a message with timestamp prefix
pub fn info<D: fmt::Display>(message: D) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    eprintln!("[{}] [INFO] {}", timestamp, message);
}

/// Log a message with timestamp prefix
pub fn warn<D: fmt::Display>(message: D) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    eprintln!("[{}] [WARN] {}", timestamp, message);
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
pub async fn warn_backoff<D: fmt::Display>(message: D) {
    let sleep = ERROR_BACKOFF.fetch_add(10, Ordering::Relaxed);
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    eprintln!("[{}] [WARN] {}", timestamp, message);
    eprintln!("[{}] Sleeping for {} seconds...", timestamp, sleep);
    tokio::time::sleep(std::time::Duration::from_secs(sleep)).await;
}
