// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;

use std::collections::HashMap;

use lcilib::db::models::{DbRepositoryId,Stack, CommitToTest};

use super::util;

pub struct Stacks {
    high_priority: Vec<(Stack, Vec<CommitToTest>)>,
    low_priority: Vec<(Stack, Vec<CommitToTest>)>,
}

impl Stacks {
    /// Returns all the high-priority and low-priority stacks across all repos.
    pub async fn find_all(
        tx: &lcilib::Transaction<'_>,
    ) -> anyhow::Result<Self> {
        // Find all stacks grouped by DB and branch
        let mut branch_map =
            HashMap::<(DbRepositoryId, String), Vec<(f64, Stack, Vec<CommitToTest>)>>::new();
        for stack in Stack::get_all(tx).await? {
            let commits = stack.id.get_commits(tx).await?;
            let priority = util::calculate_stack_priority(&commits, tx)
                .await
                .context("calculating stack priority")?;

            branch_map
                .entry((stack.repository_id, stack.target_branch.clone()))
                .or_default()
                .push((priority, stack, commits));
        }

        let mut high_priority = vec![];
        let mut low_priority = vec![];

        for mut stacks in branch_map.into_values() {
            // priority order, so pop() takes the highest-priority stack
            stacks.sort_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, s, c)) = stacks.pop() {
                high_priority.push((s, c));
            }
            low_priority.extend(stacks.into_iter().map(|(_, s, c)| (s, c)));
        }

        Ok(Self { high_priority, low_priority })
    }

    /// The high priority stack list.
    pub fn high_priority(&self) -> impl Iterator<Item = &(Stack, Vec<CommitToTest>)> {
        self.high_priority.iter()
    }

    /// The low priority stack list.
    pub fn low_priority(&self) -> impl Iterator<Item = &(Stack, Vec<CommitToTest>)> {
        self.low_priority.iter()
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
