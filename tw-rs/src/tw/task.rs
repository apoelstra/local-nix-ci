// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

struct Task {
    uuid: uuid::Uuid,
    project: String,
    repo_dir: PathBuf,
    data: TaskData,
}

enum ReviewStatus {
}

enum CiStatus {
}

enum TaskData {
    Commit {
        commit_id: String,
        /// Whether this is the merge commit for some PR (FIXME do we need this?).
        is_merge: bool,
        /// Whether this is the tip commit of some PR and should be tested in detail.
        is_tip: bool,
    },
    Pr {
        title: String,
        author: String,
        number: usize,
    },
}
