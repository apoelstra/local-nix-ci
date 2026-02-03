// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use std::error::Error;

use chrono::Utc;

use crate::tw::CommitTask;

/// Utility function for formatting the current time
fn now() -> impl fmt::Display {
    Utc::now().format("%Y-%m-%d %H:%M:%S")
}

pub struct Logger {
    task: Option<CommitTask>,
}

impl Logger {
    pub fn new() -> Self {
        Logger { task: None }
    }

    pub fn set_task(&mut self, task: Option<CommitTask>) {
        self.task = task;
    }

    pub fn newline(&self) {
        eprintln!();
    }

    fn prefix(&self) {
        if let Some(ref task) = self.task {
            let prs = task.prs();
            if prs.is_empty() {
                eprint!("[{}] [{} {:12}] ", now(), task.project(), task.commit_id());
            } else {
                eprint!("[{}] [{}", now(), task.project());
                for pr in prs {
                    eprint!(" #{}", pr);
                }
                eprint!(" {:12}] ", task.commit_id());
            }
        } else {
            eprint!("[{}] ", now())
        }
    }

    pub fn info<D: fmt::Display>(&self, args: D) {
        self.prefix();
        eprintln!("{}", args);
    }

    pub fn error<D>(&self, mut error: Option<&(dyn Error + 'static)>, args: D)
    where
        D: fmt::Display,
    {
        self.prefix();
        eprintln!("ERROR: {}", args);

        while let Some(e) = error {
            eprintln!("    caused by: {e}");
            error = e.source();
        }
    }

    pub fn warn<D: fmt::Display>(&self, args: D) {
        self.prefix();
        eprintln!("WARN: {}", args);
    }
}
