// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use std::collections::HashMap;
use std::io;
use std::io::BufRead as _;
use uuid::Uuid;
use xshell::{cmd, Shell};

use super::{CommitTask, PrTask};
use super::task::PrOrCommitTask;

#[derive(Debug)]
pub struct TaskCollection {
    commits: HashMap<Uuid, CommitTask>,
    pulls: HashMap<Uuid, PrTask>,
}

impl TaskCollection {
    pub fn new(task_shell: &Shell) -> Result<Self, TaskCollectionError> {
        let mut pulls = HashMap::new();
        let mut commits = HashMap::new();

        let output = cmd!(task_shell, "task rc.json.array=off project:local-ci export")
            .output()
            .map_err(TaskCollectionError::Shell)?;

        for json in output.stdout.lines() {
            let json = json.map_err(TaskCollectionError::Utf8)?;
            let new_task = PrOrCommitTask::from_json(&json)
                .map_err(TaskCollectionError::ParseTask)?;
            match new_task {
                PrOrCommitTask::Commit(new_task) => {
                    assert_eq!(commits.insert(*new_task.uuid(), new_task), None)
                },
                PrOrCommitTask::Pr(new_task) => {
                    assert_eq!(pulls.insert(*new_task.uuid(), new_task), None)
                },
            }
        }

        // Check that we have all the tasks we need. (We don't need to check for
        // circularity or other forms of non-DAGgedness since taskwarrior does
        // these checks for us.)
        for task in pulls.values() {
            let uuid = task.dep_uuid();
            if !commits.contains_key(uuid) {
                return Err(TaskCollectionError::MissingUuid {
                    missing: *uuid,
                    needed_by: *task.uuid(),
                });
            }
        }
        for task in commits.values() {
            if let Some(uuid) = task.dep_uuid() {
                if !commits.contains_key(uuid) {
                    return Err(TaskCollectionError::MissingUuid {
                        missing: *uuid,
                        needed_by: *task.uuid(),
                    });
                }
            }
        }
                
        Ok(TaskCollection { commits, pulls })
    }

    pub fn commits(&self) -> impl Iterator<Item = (&Uuid, &CommitTask)> {
        self.commits.iter()
    }

    pub fn pulls(&self) -> impl Iterator<Item = (&Uuid, &PrTask)> {
        self.pulls.iter()
    }
}

#[derive(Debug)]
pub enum TaskCollectionError {
    MissingUuid {
        missing: Uuid,
        needed_by: Uuid,
    },
    ParseTask(super::TaskParseError),
    Shell(xshell::Error),
    Utf8(io::Error),
}

impl fmt::Display for TaskCollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingUuid { missing, needed_by } => {
                write!(f, "Missing task UUID {} needed by task {}", missing, needed_by)
            }
            Self::ParseTask(e) => write!(f, "Failed to parse task: {}", e),
            Self::Shell(e) => write!(f, "Shell command failed: {}", e),
            Self::Utf8(e) => write!(f, "UTF-8 encoding error: {}", e),
        }
    }
}

impl std::error::Error for TaskCollectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingUuid { .. } => None,
            Self::ParseTask(e) => Some(e),
            Self::Shell(e) => Some(e),
            Self::Utf8(e) => Some(e),
        }
    }
}
