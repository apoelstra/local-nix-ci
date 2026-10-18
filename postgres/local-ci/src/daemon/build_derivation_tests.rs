// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for the daemon's error-log file writer (`save_error_to_file`).

mod save_error_to_file_tests {
    use crate::daemon::build_derivation::save_error_to_file;
    use lcilib::git::CommitId;
    use std::str::FromStr as _;

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "local-ci-savelog-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("creating scratch dir");
        dir
    }

    fn find_error_log(dir: &std::path::Path) -> std::path::PathBuf {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .expect("reading dir")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("local-ci-error-"))
            })
            .collect();
        assert_eq!(entries.len(), 1, "expected exactly one error log file");
        entries.pop().expect("just checked")
    }

    #[tokio::test]
    async fn writes_stdout_and_stderr_in_order() {
        let dir = scratch_dir("order");
        let repo_dir = dir.join("repo");
        std::fs::create_dir_all(&repo_dir).expect("repo dir");

        let commit = CommitId::from_str(&"ab".repeat(20)).expect("commit id");

        save_error_to_file(
            repo_dir.to_str().expect("utf8"),
            &commit,
            "build",
            b"stdout line 1\nstdout line 2\n",
            b"stderr line 1\n",
        )
        .await
        .expect("saving error log");

        // The log is written next to the repo directory (its parent), named
        // local-ci-error-<commit>-<op>-<timestamp>.log.
        let log_path = find_error_log(&dir);

        let contents = std::fs::read_to_string(&log_path).expect("reading log");
        let expected_prefix = format!("nix-build failed for commit {}\n\nSTDOUT:\n", commit);
        assert!(
            contents.starts_with(&expected_prefix),
            "log must start with the preamble; got: {contents:?}"
        );
        assert!(
            contents.contains("stdout line 1\nstdout line 2\n"),
            "log must contain the full stdout; got: {contents:?}"
        );
        assert!(
            contents.ends_with("\nSTDERR:\nstderr line 1\n"),
            "log must end with the stderr section; got: {contents:?}"
        );

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    /// Regression test: `save_error_to_file` used to call `fs::write` once per
    /// section, and each call truncates the file, so the preamble and stdout
    /// were destroyed and only the stderr survived.
    #[tokio::test]
    async fn large_stdout_is_not_truncated_by_stderr_write() {
        let dir = scratch_dir("bigstdout");
        let repo_dir = dir.join("repo");
        std::fs::create_dir_all(&repo_dir).expect("repo dir");

        let commit = CommitId::from_str(&"cd".repeat(20)).expect("commit id");

        // A stdout large enough that losing it would be obvious.
        let stdout: Vec<u8> = b"BUILD OUTPUT ".repeat(10_000);
        let stderr = b"error: build failed\n";

        save_error_to_file(
            repo_dir.to_str().expect("utf8"),
            &commit,
            "instantiate",
            &stdout,
            stderr,
        )
        .await
        .expect("saving error log");

        let log_path = find_error_log(&dir);

        let contents = std::fs::read_to_string(&log_path).expect("reading log");
        assert!(
            contents.contains(&"BUILD OUTPUT ".repeat(10_000)),
            "the whole stdout must survive; the file was truncated"
        );
        assert!(
            contents.ends_with("\nSTDERR:\nerror: build failed\n"),
            "stderr section must be present at the end"
        );
        // Total length sanity check: every section present exactly once.
        let expected_len = format!("nix-build failed for commit {}\n\nSTDOUT:\n", commit).len()
            + stdout.len()
            + "\nSTDERR:\n".len()
            + stderr.len();
        assert_eq!(
            contents.len(),
            expected_len,
            "log file must contain every section exactly once"
        );

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    /// The error log must be written next to the repo (its parent directory),
    /// not inside it — this is the documented location users look at.
    #[tokio::test]
    async fn writes_to_repo_parent_directory() {
        let dir = scratch_dir("parent");
        let repo_dir = dir.join("repo");
        std::fs::create_dir_all(&repo_dir).expect("repo dir");

        let commit = CommitId::from_str(&"ee".repeat(20)).expect("commit id");

        save_error_to_file(repo_dir.to_str().expect("utf8"), &commit, "build", b"", b"")
            .await
            .expect("saving error log");

        let in_parent = std::fs::read_dir(&dir)
            .expect("reading parent dir")
            .filter_map(|e| e.ok())
            .any(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("local-ci-error-"))
            });
        assert!(in_parent, "error log must be written in the repo's parent dir");
        let in_repo = std::fs::read_dir(&repo_dir)
            .expect("reading repo dir")
            .filter_map(|e| e.ok())
            .any(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("local-ci-error-"))
            });
        assert!(!in_repo, "error log must not be written inside the repo");

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }
}
