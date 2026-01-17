// SPDX-License-Identifier: GPL-3.0-or-later

pub mod collection;
pub mod serde_types;
pub mod shell;
pub mod task;

/// Re-export some things for convenience.
pub use self::collection::{TaskCollection, TaskCollectionError};
pub use self::shell::task_shell;
pub use self::task::{CommitTask, PrTask, TaskParseError};
