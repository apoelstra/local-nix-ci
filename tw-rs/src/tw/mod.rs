// SPDX-License-Identifier: GPL-3.0-or-later

pub mod collection;
pub mod shell;
pub mod task;

/// Re-export some things for convenience.
pub use self::collection::{TaskCollection, TaskCollectionError};
pub use self::task::{CiStatus, CommitTask, ReviewStatus, PrTask, TaskParseError};
pub use self::shell::task_shell;
