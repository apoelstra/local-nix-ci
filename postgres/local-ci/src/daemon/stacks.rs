// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use std::collections::HashMap;

use lcilib::db::{DbQueryError, models::{CommitToTest, DbRepositoryId, DbStackId, Stack}};

use super::util;

#[derive(Debug)]
pub enum Error {
    GetStacks(DbQueryError),
    GetCommits {
        e: DbQueryError,
        id: DbStackId,
    },
    Priority {
        e: util::PriorityError,
        id: DbStackId,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::GetStacks(_) => f.write_str("getting stacks from database"),
            Self::GetCommits { id, .. } => {
                write!(f, "getting commits from database for stack with ID {id}")
            },
            Self::Priority { id, .. } => {
                write!(f, "calculating priority for stack with database ID {id}")
            },
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::GetStacks(ref e) => Some(e),
            Self::GetCommits { ref e, .. } => Some(e),
            Self::Priority { ref e, .. } => Some(e),
        }
    }
}

pub struct Stacks {
    inner: Vec<(Stack, Vec<CommitToTest>)>,
    high_priority: Vec<usize>,
    low_priority: Vec<usize>,
}

impl Stacks {
    /// Returns all the high-priority and low-priority stacks across all repos.
    pub async fn find_all(
        tx: &lcilib::Transaction<'_>,
    ) -> Result<Self, Error> {
        let mut inner = vec![];
        // Find all stacks grouped by DB and branch
        let mut branch_map =
            HashMap::<(DbRepositoryId, String), Vec<(f64, usize)>>::new();
        for stack in Stack::get_all(tx).await.map_err(Error::GetStacks)? {
            let commits = stack.id.get_commits(tx)
                .await
                .map_err(|e| Error::GetCommits { e, id: stack.id })?;
            let priority = util::calculate_stack_priority(&commits, tx)
                .await
                .map_err(|e| Error::Priority { e, id: stack.id })?;

            branch_map
                .entry((stack.repository_id, stack.target_branch.clone()))
                .or_default()
                .push((priority, inner.len()));

            inner.push((stack, commits));
        }

        let mut high_priority = vec![];
        let mut low_priority = vec![];

        for mut stacks in branch_map.into_values() {
            // priority order, so pop() takes the highest-priority stack
            stacks.sort_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, idx)) = stacks.pop() {
                high_priority.push(idx);
            }
            low_priority.extend(stacks.into_iter().map(|(_, idx)| idx));
        }

        Ok(Self { inner, high_priority, low_priority })
    }

    /// The high priority stack list.
    pub fn high_priority(&self) -> impl Iterator<Item = &(Stack, Vec<CommitToTest>)> {
        self.high_priority.iter().map(|idx| &self.inner[*idx])
    }

    /// The low priority stack list.
    pub fn low_priority(&self) -> impl Iterator<Item = &(Stack, Vec<CommitToTest>)> {
        self.low_priority.iter().map(|idx| &self.inner[*idx])
    }

    /// Whether the "high priority" stack list is empty.
    pub fn low_priority_is_empty(&self) -> bool {
        self.high_priority.is_empty()
    }

    /// Whether the "high priority" stack list is empty.
    pub fn high_priority_is_empty(&self) -> bool {
        self.high_priority.is_empty()
    }

    /// Whether both stack lists are empty.
    pub fn is_empty(&self) -> bool {
        self.high_priority.is_empty() && self.low_priority.is_empty()
    }
}
