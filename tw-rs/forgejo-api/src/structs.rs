
use crate::reqwest;
use crate::url;

use crate::{impl_from_response, StructureError};
use std::collections::BTreeMap;
/// APIError is an api error with a message
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct APIError {
    pub message: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(APIError);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct APIForbiddenError {
    pub message: Option<String>,
    pub url: Option<String>,
}

impl_from_response!(APIForbiddenError);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct APIInternalServerError {
    pub message: Option<String>,
    pub url: Option<String>,
}

impl_from_response!(APIInternalServerError);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct APIInvalidTopicsError {
    #[serde(rename = "invalidTopics")]
    pub invalid_topics: Option<Vec<String>>,
    pub message: Option<String>,
}

impl_from_response!(APIInvalidTopicsError);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct APINotFound {
    pub errors: Option<Vec<String>>,
    pub message: Option<String>,
    pub url: Option<String>,
}

impl_from_response!(APINotFound);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct APIRepoArchivedError {
    pub message: Option<String>,
    pub url: Option<String>,
}

impl_from_response!(APIRepoArchivedError);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct APIUnauthorizedError {
    pub message: Option<String>,
    pub url: Option<String>,
}

impl_from_response!(APIUnauthorizedError);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct APIValidationError {
    pub message: Option<String>,
    pub url: Option<String>,
}

impl_from_response!(APIValidationError);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AccessToken {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub sha1: Option<String>,
    pub token_last_eight: Option<String>,
}

impl_from_response!(AccessToken);

/// ActionRun represents an action run
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActionRun {
    /// the cron id for the schedule trigger
    #[serde(rename = "ScheduleID")]
    pub schedule_id: Option<i64>,
    /// who approved this action run
    pub approved_by: Option<i64>,
    /// the commit sha the action run ran on
    pub commit_sha: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    /// when the action run was created
    pub created: Option<time::OffsetDateTime>,
    pub duration: Option<i64>,
    /// the webhook event that causes the workflow to run
    pub event: Option<String>,
    /// the payload of the webhook event that causes the workflow to run
    pub event_payload: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// the url of this action run
    pub html_url: Option<url::Url>,
    /// the action run id
    pub id: Option<i64>,
    /// a unique number for each run of a repository
    pub index_in_repo: Option<i64>,
    /// If this is triggered by a PR from a forked repository or an untrusted user, we need to check if it is approved and limit permissions when running the workflow.
    pub is_fork_pull_request: Option<bool>,
    /// has the commit/tag/… the action run ran on been deleted
    pub is_ref_deleted: Option<bool>,
    /// may need approval if it's a fork pull request
    pub need_approval: Option<bool>,
    /// the commit/tag/… the action run ran on
    pub prettyref: Option<String>,
    pub repository: Option<Repository>,
    #[serde(with = "time::serde::rfc3339::option")]
    /// when the action run was started
    pub started: Option<time::OffsetDateTime>,
    /// the current status of this run
    pub status: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    /// when the action run was stopped
    pub stopped: Option<time::OffsetDateTime>,
    /// the action run's title
    pub title: Option<String>,
    /// the trigger event defined in the `on` configuration of the triggered workflow
    pub trigger_event: Option<String>,
    pub trigger_user: Option<User>,
    #[serde(with = "time::serde::rfc3339::option")]
    /// when the action run was last updated
    pub updated: Option<time::OffsetDateTime>,
    /// the name of workflow file
    pub workflow_id: Option<String>,
}

impl_from_response!(ActionRun);

/// ActionRunJob represents a job of a run
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActionRunJob {
    /// the action run job id
    pub id: Option<i64>,
    /// the action run job name
    pub name: Option<String>,
    /// the action run job needed ids
    pub needs: Option<Vec<String>>,
    /// the owner id
    pub owner_id: Option<i64>,
    /// the repository id
    pub repo_id: Option<i64>,
    /// the action run job labels to run on
    pub runs_on: Option<Vec<String>>,
    /// the action run job status
    pub status: Option<String>,
    /// the action run job latest task id
    pub task_id: Option<i64>,
}

impl_from_response!(ActionRunJob);

/// ActionTask represents a ActionTask
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActionTask {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub display_title: Option<String>,
    pub event: Option<String>,
    pub head_branch: Option<String>,
    pub head_sha: Option<String>,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub run_number: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub run_started_at: Option<time::OffsetDateTime>,
    pub status: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    pub workflow_id: Option<String>,
}

/// ActionTaskResponse returns a ActionTask
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActionTaskResponse {
    pub total_count: Option<i64>,
    pub workflow_runs: Option<Vec<ActionTask>>,
}

impl_from_response!(ActionTaskResponse);

/// ActionVariable return value of the query API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActionVariable {
    /// the value of the variable
    pub data: Option<String>,
    /// the name of the variable
    pub name: Option<String>,
    /// the owner to which the variable belongs
    pub owner_id: Option<i64>,
    /// the repository to which the variable belongs
    pub repo_id: Option<i64>,
}

impl_from_response!(ActionVariable);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Activity {
    pub act_user: Option<User>,
    pub act_user_id: Option<i64>,
    pub comment: Option<Comment>,
    pub comment_id: Option<i64>,
    pub content: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    pub id: Option<i64>,
    pub is_private: Option<bool>,
    /// the type of action
    pub op_type: Option<ActivityOpType>,
    pub ref_name: Option<String>,
    pub repo: Option<Repository>,
    pub repo_id: Option<i64>,
    pub user_id: Option<i64>,
}

impl_from_response!(Activity);

/// the type of action

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActivityOpType {
    #[serde(rename = "create_repo")]
    CreateRepo,
    #[serde(rename = "rename_repo")]
    RenameRepo,
    #[serde(rename = "star_repo")]
    StarRepo,
    #[serde(rename = "watch_repo")]
    WatchRepo,
    #[serde(rename = "commit_repo")]
    CommitRepo,
    #[serde(rename = "create_issue")]
    CreateIssue,
    #[serde(rename = "create_pull_request")]
    CreatePullRequest,
    #[serde(rename = "transfer_repo")]
    TransferRepo,
    #[serde(rename = "push_tag")]
    PushTag,
    #[serde(rename = "comment_issue")]
    CommentIssue,
    #[serde(rename = "merge_pull_request")]
    MergePullRequest,
    #[serde(rename = "close_issue")]
    CloseIssue,
    #[serde(rename = "reopen_issue")]
    ReopenIssue,
    #[serde(rename = "close_pull_request")]
    ClosePullRequest,
    #[serde(rename = "reopen_pull_request")]
    ReopenPullRequest,
    #[serde(rename = "delete_tag")]
    DeleteTag,
    #[serde(rename = "delete_branch")]
    DeleteBranch,
    #[serde(rename = "mirror_sync_push")]
    MirrorSyncPush,
    #[serde(rename = "mirror_sync_create")]
    MirrorSyncCreate,
    #[serde(rename = "mirror_sync_delete")]
    MirrorSyncDelete,
    #[serde(rename = "approve_pull_request")]
    ApprovePullRequest,
    #[serde(rename = "reject_pull_request")]
    RejectPullRequest,
    #[serde(rename = "comment_pull")]
    CommentPull,
    #[serde(rename = "publish_release")]
    PublishRelease,
    #[serde(rename = "pull_review_dismissed")]
    PullReviewDismissed,
    #[serde(rename = "pull_request_ready_for_review")]
    PullRequestReadyForReview,
    #[serde(rename = "auto_merge_pull_request")]
    AutoMergePullRequest,
}
/// ActivityPub type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActivityPub {
    #[serde(rename = "@context")]
    pub context: Option<String>,
}

impl_from_response!(ActivityPub);

/// AddCollaboratorOption options when adding a user as a collaborator of a repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AddCollaboratorOption {
    pub permission: Option<AddCollaboratorOptionPermission>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AddCollaboratorOptionPermission {
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "admin")]
    Admin,
}
/// AddTimeOption options for adding time to an issue
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AddTimeOption {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    /// time in seconds
    pub time: i64,
    /// User who spent the time (optional)
    pub user_name: Option<String>,
}

/// AnnotatedTag represents an annotated tag
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnnotatedTag {
    pub archive_download_count: Option<TagArchiveDownloadCount>,
    pub message: Option<String>,
    pub object: Option<AnnotatedTagObject>,
    pub sha: Option<String>,
    pub tag: Option<String>,
    pub tagger: Option<CommitUser>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    pub verification: Option<PayloadCommitVerification>,
}

impl_from_response!(AnnotatedTag);

/// AnnotatedTagObject contains meta information of the tag object
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnnotatedTagObject {
    pub sha: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

/// Attachment a generic attachment
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Attachment {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub browser_download_url: Option<url::Url>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub download_count: Option<i64>,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub size: Option<i64>,
    #[serde(rename = "type")]
    pub r#type: Option<AttachmentType>,
    pub uuid: Option<String>,
}

impl_from_response!(Attachment);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AttachmentType {
    #[serde(rename = "attachment")]
    Attachment,
    #[serde(rename = "external")]
    External,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BlockedUser {
    pub block_id: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
}

impl_from_response!(BlockedUser);

/// Branch represents a repository branch
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Branch {
    pub commit: Option<PayloadCommit>,
    pub effective_branch_protection_name: Option<String>,
    pub enable_status_check: Option<bool>,
    pub name: Option<String>,
    pub protected: Option<bool>,
    pub required_approvals: Option<i64>,
    pub status_check_contexts: Option<Vec<String>>,
    pub user_can_merge: Option<bool>,
    pub user_can_push: Option<bool>,
}

impl_from_response!(Branch);

/// BranchProtection represents a branch protection for a repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BranchProtection {
    pub apply_to_admins: Option<bool>,
    pub approvals_whitelist_teams: Option<Vec<String>>,
    pub approvals_whitelist_username: Option<Vec<String>>,
    pub block_on_official_review_requests: Option<bool>,
    pub block_on_outdated_branch: Option<bool>,
    pub block_on_rejected_reviews: Option<bool>,
    /// Deprecated: true
    pub branch_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub dismiss_stale_approvals: Option<bool>,
    pub enable_approvals_whitelist: Option<bool>,
    pub enable_merge_whitelist: Option<bool>,
    pub enable_push: Option<bool>,
    pub enable_push_whitelist: Option<bool>,
    pub enable_status_check: Option<bool>,
    pub ignore_stale_approvals: Option<bool>,
    pub merge_whitelist_teams: Option<Vec<String>>,
    pub merge_whitelist_usernames: Option<Vec<String>>,
    pub protected_file_patterns: Option<String>,
    pub push_whitelist_deploy_keys: Option<bool>,
    pub push_whitelist_teams: Option<Vec<String>>,
    pub push_whitelist_usernames: Option<Vec<String>>,
    pub require_signed_commits: Option<bool>,
    pub required_approvals: Option<i64>,
    pub rule_name: Option<String>,
    pub status_check_contexts: Option<Vec<String>>,
    pub unprotected_file_patterns: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
}

impl_from_response!(BranchProtection);

/// ChangeFileOperation for creating, updating or deleting a file
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChangeFileOperation {
    /// new or updated file content, must be base64 encoded
    pub content: Option<String>,
    /// old path of the file to move
    pub from_path: Option<String>,
    /// indicates what to do with the file
    pub operation: ChangeFileOperationOperation,
    /// path to the existing or new file
    pub path: String,
    /// sha is the SHA for the file that already exists, required for update or delete
    pub sha: Option<String>,
}

/// indicates what to do with the file

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChangeFileOperationOperation {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "delete")]
    Delete,
}
/// ChangeFilesOptions options for creating, updating or deleting multiple files
///
/// Note: `author` and `committer` are optional (if only one is given, it will be used for the other, otherwise the authenticated user will be used)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChangeFilesOptions {
    pub author: Option<Identity>,
    /// branch (optional) to base this file from. if not given, the default branch is used
    pub branch: Option<String>,
    pub committer: Option<Identity>,
    pub dates: Option<CommitDateOptions>,
    /// list of file operations
    pub files: Vec<ChangeFileOperation>,
    /// message (optional) for the commit of this file. if not supplied, a default message will be used
    pub message: Option<String>,
    /// new_branch (optional) will make a new branch from `branch` before creating the file
    pub new_branch: Option<String>,
    /// Add a Signed-off-by trailer by the committer at the end of the commit log message.
    pub signoff: Option<bool>,
}

/// ChangedFile store information about files affected by the pull request
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChangedFile {
    pub additions: Option<i64>,
    pub changes: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub contents_url: Option<url::Url>,
    pub deletions: Option<i64>,
    pub filename: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub previous_filename: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub raw_url: Option<url::Url>,
    pub status: Option<String>,
}

impl_from_response!(ChangedFile);

/// CombinedStatus holds the combined state of several statuses for a single commit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CombinedStatus {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub commit_url: Option<url::Url>,
    pub repository: Option<Repository>,
    pub sha: Option<String>,
    pub state: Option<CommitStatusState>,
    pub statuses: Option<Vec<CommitStatus>>,
    pub total_count: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(CombinedStatus);

impl crate::sealed::Sealed for CombinedStatus {}
impl crate::PageSize for CombinedStatus {
    fn page_size(&self) -> usize {
        self.statuses.as_ref().map(|x| x.page_size()).unwrap_or(0)
    }
}

/// Comment represents a comment on a commit or issue
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Comment {
    pub assets: Option<Vec<Attachment>>,
    pub body: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub issue_url: Option<url::Url>,
    pub original_author: Option<String>,
    pub original_author_id: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub pull_request_url: Option<url::Url>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    pub user: Option<User>,
}

impl_from_response!(Comment);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Commit {
    pub author: Option<User>,
    pub commit: Option<RepoCommit>,
    pub committer: Option<User>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    pub files: Option<Vec<CommitAffectedFiles>>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub parents: Option<Vec<CommitMeta>>,
    pub sha: Option<String>,
    pub stats: Option<CommitStats>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(Commit);

/// CommitAffectedFiles store information about files affected by the commit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommitAffectedFiles {
    pub filename: Option<String>,
    pub status: Option<String>,
}

/// CommitDateOptions store dates for GIT_AUTHOR_DATE and GIT_COMMITTER_DATE
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommitDateOptions {
    #[serde(with = "time::serde::rfc3339::option")]
    pub author: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub committer: Option<time::OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommitMeta {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    pub sha: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

/// CommitStats is statistics for a RepoCommit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommitStats {
    pub additions: Option<i64>,
    pub deletions: Option<i64>,
    pub total: Option<i64>,
}

/// CommitStatus holds a single status of a single Commit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommitStatus {
    pub context: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub creator: Option<User>,
    pub description: Option<String>,
    pub id: Option<i64>,
    pub status: Option<CommitStatusState>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub target_url: Option<url::Url>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(CommitStatus);

/// CommitStatusState holds the state of a CommitStatus
///
/// It can be "pending", "success", "error", "failure" and "warning"

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CommitStatusState {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "failure")]
    Failure,
    #[serde(rename = "warning")]
    Warning,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommitUser {
    #[serde(with = "time::serde::rfc3339::option")]
    pub date: Option<time::OffsetDateTime>,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Compare {
    pub commits: Option<Vec<Commit>>,
    pub files: Option<Vec<CommitAffectedFiles>>,
    pub total_commits: Option<i64>,
}

impl_from_response!(Compare);

/// ContentsResponse contains information about a repo's entry's (dir, file, symlink, submodule) metadata and content
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContentsResponse {
    #[serde(rename = "_links")]
    pub links: Option<FileLinksResponse>,
    /// `content` is populated when `type` is `file`, otherwise null
    pub content: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub download_url: Option<url::Url>,
    /// `encoding` is populated when `type` is `file`, otherwise null
    pub encoding: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub git_url: Option<url::Url>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub last_commit_sha: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_commit_when: Option<time::OffsetDateTime>,
    pub name: Option<String>,
    pub path: Option<String>,
    pub sha: Option<String>,
    pub size: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// `submodule_git_url` is populated when `type` is `submodule`, otherwise null
    pub submodule_git_url: Option<url::Url>,
    /// `target` is populated when `type` is `symlink`, otherwise null
    pub target: Option<String>,
    /// `type` will be `file`, `dir`, `symlink`, or `submodule`
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(ContentsResponse);

/// CreateAccessTokenOption options when create access token
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateAccessTokenOption {
    pub name: String,
    pub scopes: Option<Vec<String>>,
}

/// CreateBranchProtectionOption options for creating a branch protection
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateBranchProtectionOption {
    pub apply_to_admins: Option<bool>,
    pub approvals_whitelist_teams: Option<Vec<String>>,
    pub approvals_whitelist_username: Option<Vec<String>>,
    pub block_on_official_review_requests: Option<bool>,
    pub block_on_outdated_branch: Option<bool>,
    pub block_on_rejected_reviews: Option<bool>,
    /// Deprecated: true
    pub branch_name: Option<String>,
    pub dismiss_stale_approvals: Option<bool>,
    pub enable_approvals_whitelist: Option<bool>,
    pub enable_merge_whitelist: Option<bool>,
    pub enable_push: Option<bool>,
    pub enable_push_whitelist: Option<bool>,
    pub enable_status_check: Option<bool>,
    pub ignore_stale_approvals: Option<bool>,
    pub merge_whitelist_teams: Option<Vec<String>>,
    pub merge_whitelist_usernames: Option<Vec<String>>,
    pub protected_file_patterns: Option<String>,
    pub push_whitelist_deploy_keys: Option<bool>,
    pub push_whitelist_teams: Option<Vec<String>>,
    pub push_whitelist_usernames: Option<Vec<String>>,
    pub require_signed_commits: Option<bool>,
    pub required_approvals: Option<i64>,
    pub rule_name: Option<String>,
    pub status_check_contexts: Option<Vec<String>>,
    pub unprotected_file_patterns: Option<String>,
}

/// CreateBranchRepoOption options when creating a branch in a repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateBranchRepoOption {
    /// Name of the branch to create
    pub new_branch_name: String,
    /// Deprecated: true
    ///
    /// Name of the old branch to create from
    pub old_branch_name: Option<String>,
    /// Name of the old branch/tag/commit to create from
    pub old_ref_name: Option<String>,
}

/// CreateEmailOption options when creating email addresses
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateEmailOption {
    /// email addresses to add
    pub emails: Option<Vec<String>>,
}

/// CreateFileOptions options for creating files
///
/// Note: `author` and `committer` are optional (if only one is given, it will be used for the other, otherwise the authenticated user will be used)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateFileOptions {
    pub author: Option<Identity>,
    /// branch (optional) to base this file from. if not given, the default branch is used
    pub branch: Option<String>,
    pub committer: Option<Identity>,
    /// content must be base64 encoded
    pub content: String,
    pub dates: Option<CommitDateOptions>,
    /// message (optional) for the commit of this file. if not supplied, a default message will be used
    pub message: Option<String>,
    /// new_branch (optional) will make a new branch from `branch` before creating the file
    pub new_branch: Option<String>,
    /// Add a Signed-off-by trailer by the committer at the end of the commit log message.
    pub signoff: Option<bool>,
}

/// CreateForkOption options for creating a fork
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateForkOption {
    /// name of the forked repository
    pub name: Option<String>,
    /// organization name, if forking into an organization
    pub organization: Option<String>,
}

/// CreateGPGKeyOption options create user GPG key
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateGPGKeyOption {
    /// An armored GPG key to add
    pub armored_public_key: String,
    pub armored_signature: Option<String>,
}

/// CreateHookOption options when create a hook
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateHookOption {
    pub active: Option<bool>,
    pub authorization_header: Option<String>,
    pub branch_filter: Option<String>,
    pub config: CreateHookOptionConfig,
    pub events: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub r#type: CreateHookOptionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CreateHookOptionType {
    #[serde(rename = "forgejo")]
    Forgejo,
    #[serde(rename = "dingtalk")]
    Dingtalk,
    #[serde(rename = "discord")]
    Discord,
    #[serde(rename = "gitea")]
    Gitea,
    #[serde(rename = "gogs")]
    Gogs,
    #[serde(rename = "msteams")]
    Msteams,
    #[serde(rename = "slack")]
    Slack,
    #[serde(rename = "telegram")]
    Telegram,
    #[serde(rename = "feishu")]
    Feishu,
    #[serde(rename = "wechatwork")]
    Wechatwork,
    #[serde(rename = "packagist")]
    Packagist,
}
/// CreateHookOptionConfig has all config options in it
///
/// required are "content_type" and "url" Required
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateHookOptionConfig {
    pub content_type: String,
    pub url: url::Url,
    #[serde(flatten)]
    pub additional: BTreeMap<String, String>,
}

/// CreateIssueCommentOption options for creating a comment on an issue
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateIssueCommentOption {
    pub body: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
}

/// CreateIssueOption options to create one issue
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateIssueOption {
    /// deprecated
    pub assignee: Option<String>,
    pub assignees: Option<Vec<String>>,
    pub body: Option<String>,
    pub closed: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_date: Option<time::OffsetDateTime>,
    /// list of label ids
    pub labels: Option<Vec<i64>>,
    /// milestone id
    pub milestone: Option<i64>,
    #[serde(rename = "ref")]
    pub r#ref: Option<String>,
    pub title: String,
}

/// CreateKeyOption options when creating a key
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateKeyOption {
    /// An armored SSH key to add
    pub key: String,
    /// Describe if the key has only read access or read/write
    pub read_only: Option<bool>,
    /// Title of the key to add
    pub title: String,
}

/// CreateLabelOption options for creating a label
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateLabelOption {
    pub color: String,
    pub description: Option<String>,
    pub exclusive: Option<bool>,
    pub is_archived: Option<bool>,
    pub name: String,
}

/// CreateMilestoneOption options for creating a milestone
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateMilestoneOption {
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_on: Option<time::OffsetDateTime>,
    pub state: Option<CreateMilestoneOptionState>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CreateMilestoneOptionState {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
}
/// CreateOAuth2ApplicationOptions holds options to create an oauth2 application
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateOAuth2ApplicationOptions {
    pub confidential_client: Option<bool>,
    pub name: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
}

/// CreateOrUpdateSecretOption options when creating or updating secret
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateOrUpdateSecretOption {
    /// Data of the secret to update
    pub data: String,
}

/// CreateOrgOption options for creating an organization
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateOrgOption {
    pub description: Option<String>,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub location: Option<String>,
    pub repo_admin_change_team_access: Option<bool>,
    pub username: String,
    /// possible values are `public` (default), `limited` or `private`
    pub visibility: Option<CreateOrgOptionVisibility>,
    pub website: Option<String>,
}

/// possible values are `public` (default), `limited` or `private`

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CreateOrgOptionVisibility {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "limited")]
    Limited,
    #[serde(rename = "private")]
    Private,
}
/// CreatePullRequestOption options when creating a pull request
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreatePullRequestOption {
    pub assignee: Option<String>,
    pub assignees: Option<Vec<String>>,
    pub base: Option<String>,
    pub body: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_date: Option<time::OffsetDateTime>,
    pub head: Option<String>,
    pub labels: Option<Vec<i64>>,
    pub milestone: Option<i64>,
    pub title: Option<String>,
}

/// CreatePullReviewComment represent a review comment for creation api
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreatePullReviewComment {
    pub body: Option<String>,
    /// if comment to new file line or 0
    pub new_position: Option<i64>,
    /// if comment to old file line or 0
    pub old_position: Option<i64>,
    /// the tree path
    pub path: Option<String>,
}

/// CreatePullReviewOptions are options to create a pull review
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreatePullReviewOptions {
    pub body: Option<String>,
    pub comments: Option<Vec<CreatePullReviewComment>>,
    pub commit_id: Option<String>,
    pub event: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreatePushMirrorOption {
    pub branch_filter: Option<String>,
    pub interval: Option<String>,
    pub remote_address: Option<String>,
    pub remote_password: Option<String>,
    pub remote_username: Option<String>,
    pub sync_on_commit: Option<bool>,
    pub use_ssh: Option<bool>,
}

/// CreateQutaGroupOptions represents the options for creating a quota group
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateQuotaGroupOptions {
    /// Name of the quota group to create
    pub name: Option<String>,
    /// Rules to add to the newly created group.
    ///
    /// If a rule does not exist, it will be created.
    pub rules: Option<Vec<CreateQuotaRuleOptions>>,
}

/// CreateQuotaRuleOptions represents the options for creating a quota rule
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateQuotaRuleOptions {
    /// The limit set by the rule
    pub limit: Option<i64>,
    /// Name of the rule to create
    pub name: Option<String>,
    /// The subjects affected by the rule
    pub subjects: Option<Vec<CreateQuotaRuleOptionsSubjects>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CreateQuotaRuleOptionsSubjects {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "size:all")]
    SizeAll,
    #[serde(rename = "size:repos:all")]
    SizeReposAll,
    #[serde(rename = "size:repos:public")]
    SizeReposPublic,
    #[serde(rename = "size:repos:private")]
    SizeReposPrivate,
    #[serde(rename = "size:git:all")]
    SizeGitAll,
    #[serde(rename = "size:git:lfs")]
    SizeGitLfs,
    #[serde(rename = "size:assets:all")]
    SizeAssetsAll,
    #[serde(rename = "size:assets:attachments:all")]
    SizeAssetsAttachmentsAll,
    #[serde(rename = "size:assets:attachments:issues")]
    SizeAssetsAttachmentsIssues,
    #[serde(rename = "size:assets:attachments:releases")]
    SizeAssetsAttachmentsReleases,
    #[serde(rename = "size:assets:artifacts")]
    SizeAssetsArtifacts,
    #[serde(rename = "size:assets:packages:all")]
    SizeAssetsPackagesAll,
    #[serde(rename = "size:assets:wiki")]
    SizeAssetsWiki,
}
/// CreateReleaseOption options when creating a release
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateReleaseOption {
    pub body: Option<String>,
    pub draft: Option<bool>,
    pub hide_archive_links: Option<bool>,
    pub name: Option<String>,
    pub prerelease: Option<bool>,
    pub tag_name: String,
    pub target_commitish: Option<String>,
}

/// CreateRepoOption options when creating repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateRepoOption {
    /// Whether the repository should be auto-initialized?
    pub auto_init: Option<bool>,
    /// DefaultBranch of the repository (used when initializes and in template)
    pub default_branch: Option<String>,
    /// Description of the repository to create
    pub description: Option<String>,
    /// Gitignores to use
    pub gitignores: Option<String>,
    /// Label-Set to use
    pub issue_labels: Option<String>,
    /// License to use
    pub license: Option<String>,
    /// Name of the repository to create
    pub name: String,
    pub object_format_name: Option<ObjectFormatName>,
    /// Whether the repository is private
    pub private: Option<bool>,
    /// Readme of the repository to create
    pub readme: Option<String>,
    /// Whether the repository is template
    pub template: Option<bool>,
    /// TrustModel of the repository
    pub trust_model: Option<CreateRepoOptionTrustModel>,
}

/// TrustModel of the repository

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CreateRepoOptionTrustModel {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "collaborator")]
    Collaborator,
    #[serde(rename = "committer")]
    Committer,
    #[serde(rename = "collaboratorcommitter")]
    Collaboratorcommitter,
}
/// CreateStatusOption holds the information needed to create a new CommitStatus for a Commit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateStatusOption {
    pub context: Option<String>,
    pub description: Option<String>,
    pub state: Option<CommitStatusState>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub target_url: Option<url::Url>,
}

/// CreateTagOption options when creating a tag
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateTagOption {
    pub message: Option<String>,
    pub tag_name: String,
    pub target: Option<String>,
}

/// CreateTagProtectionOption options for creating a tag protection
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateTagProtectionOption {
    pub name_pattern: Option<String>,
    pub whitelist_teams: Option<Vec<String>>,
    pub whitelist_usernames: Option<Vec<String>>,
}

/// CreateTeamOption options for creating a team
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateTeamOption {
    pub can_create_org_repo: Option<bool>,
    pub description: Option<String>,
    pub includes_all_repositories: Option<bool>,
    pub name: String,
    pub permission: Option<CreateTeamOptionPermission>,
    pub units: Option<Vec<String>>,
    pub units_map: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CreateTeamOptionPermission {
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "admin")]
    Admin,
}
/// CreateUserOption create user options
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateUserOption {
    #[serde(with = "time::serde::rfc3339::option")]
    /// For explicitly setting the user creation timestamp. Useful when users are
    ///
    /// migrated from other systems. When omitted, the user's creation timestamp
    ///
    /// will be set to "now".
    pub created_at: Option<time::OffsetDateTime>,
    pub email: String,
    pub full_name: Option<String>,
    pub login_name: Option<String>,
    pub must_change_password: Option<bool>,
    pub password: Option<String>,
    pub restricted: Option<bool>,
    pub send_notify: Option<bool>,
    pub source_id: Option<i64>,
    pub username: String,
    pub visibility: Option<String>,
}

/// CreateVariableOption the option when creating variable
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateVariableOption {
    /// Value of the variable to create
    pub value: String,
}

/// CreateWikiPageOptions form for creating wiki
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateWikiPageOptions {
    /// content must be base64 encoded
    pub content_base64: Option<String>,
    /// optional commit message summarizing the change
    pub message: Option<String>,
    /// page title. leave empty to keep unchanged
    pub title: Option<String>,
}

/// Cron represents a Cron task
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cron {
    pub exec_times: Option<i64>,
    pub name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub next: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub prev: Option<time::OffsetDateTime>,
    pub schedule: Option<String>,
}

impl_from_response!(Cron);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DefaultMergeStyle {
    #[serde(rename = "merge")]
    Merge,
    #[serde(rename = "rebase")]
    Rebase,
    #[serde(rename = "rebase-merge")]
    RebaseMerge,
    #[serde(rename = "squash")]
    Squash,
    #[serde(rename = "fast-forward-only")]
    FastForwardOnly,
}
/// DeleteEmailOption options when deleting email addresses
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeleteEmailOption {
    /// email addresses to delete
    pub emails: Option<Vec<String>>,
}

/// DeleteFileOptions options for deleting files (used for other File structs below)
///
/// Note: `author` and `committer` are optional (if only one is given, it will be used for the other, otherwise the authenticated user will be used)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeleteFileOptions {
    pub author: Option<Identity>,
    /// branch (optional) to base this file from. if not given, the default branch is used
    pub branch: Option<String>,
    pub committer: Option<Identity>,
    pub dates: Option<CommitDateOptions>,
    /// message (optional) for the commit of this file. if not supplied, a default message will be used
    pub message: Option<String>,
    /// new_branch (optional) will make a new branch from `branch` before creating the file
    pub new_branch: Option<String>,
    /// sha is the SHA for the file that already exists
    pub sha: String,
    /// Add a Signed-off-by trailer by the committer at the end of the commit log message.
    pub signoff: Option<bool>,
}

/// DeleteLabelOption options for deleting a label
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeleteLabelsOption {
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
}

/// DeployKey a deploy key
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeployKey {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub fingerprint: Option<String>,
    pub id: Option<i64>,
    pub key: Option<String>,
    pub key_id: Option<i64>,
    pub read_only: Option<bool>,
    pub repository: Option<Repository>,
    pub title: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(DeployKey);

/// DismissPullReviewOptions are options to dismiss a pull review
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DismissPullReviewOptions {
    pub message: Option<String>,
    pub priors: Option<bool>,
}

/// DispatchWorkflowOption options when dispatching a workflow
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DispatchWorkflowOption {
    /// Input keys and values configured in the workflow file.
    pub inputs: Option<BTreeMap<String, String>>,
    /// Git reference for the workflow
    #[serde(rename = "ref")]
    pub r#ref: String,
    /// Flag to return the run info
    pub return_run_info: Option<bool>,
}

/// DispatchWorkflowRun represents a workflow run
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DispatchWorkflowRun {
    /// the workflow run id
    pub id: Option<i64>,
    /// the jobs name
    pub jobs: Option<Vec<String>>,
    /// a unique number for each run of a repository
    pub run_number: Option<i64>,
}

impl_from_response!(DispatchWorkflowRun);

/// EditAttachmentOptions options for editing attachments
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditAttachmentOptions {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// (Can only be set if existing attachment is of external type)
    pub browser_download_url: Option<url::Url>,
    pub name: Option<String>,
}

/// EditBranchProtectionOption options for editing a branch protection
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditBranchProtectionOption {
    pub apply_to_admins: Option<bool>,
    pub approvals_whitelist_teams: Option<Vec<String>>,
    pub approvals_whitelist_username: Option<Vec<String>>,
    pub block_on_official_review_requests: Option<bool>,
    pub block_on_outdated_branch: Option<bool>,
    pub block_on_rejected_reviews: Option<bool>,
    pub dismiss_stale_approvals: Option<bool>,
    pub enable_approvals_whitelist: Option<bool>,
    pub enable_merge_whitelist: Option<bool>,
    pub enable_push: Option<bool>,
    pub enable_push_whitelist: Option<bool>,
    pub enable_status_check: Option<bool>,
    pub ignore_stale_approvals: Option<bool>,
    pub merge_whitelist_teams: Option<Vec<String>>,
    pub merge_whitelist_usernames: Option<Vec<String>>,
    pub protected_file_patterns: Option<String>,
    pub push_whitelist_deploy_keys: Option<bool>,
    pub push_whitelist_teams: Option<Vec<String>>,
    pub push_whitelist_usernames: Option<Vec<String>>,
    pub require_signed_commits: Option<bool>,
    pub required_approvals: Option<i64>,
    pub status_check_contexts: Option<Vec<String>>,
    pub unprotected_file_patterns: Option<String>,
}

/// EditDeadlineOption options for creating a deadline
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditDeadlineOption {
    #[serde(with = "time::serde::rfc3339")]
    pub due_date: time::OffsetDateTime,
}

/// EditGitHookOption options when modifying one Git hook
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditGitHookOption {
    pub content: Option<String>,
}

/// EditHookOption options when modify one hook
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditHookOption {
    pub active: Option<bool>,
    pub authorization_header: Option<String>,
    pub branch_filter: Option<String>,
    pub config: Option<BTreeMap<String, String>>,
    pub events: Option<Vec<String>>,
}

/// EditIssueCommentOption options for editing a comment
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditIssueCommentOption {
    pub body: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
}

/// EditIssueOption options for editing an issue
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditIssueOption {
    /// deprecated
    pub assignee: Option<String>,
    pub assignees: Option<Vec<String>>,
    pub body: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_date: Option<time::OffsetDateTime>,
    pub milestone: Option<i64>,
    #[serde(rename = "ref")]
    pub r#ref: Option<String>,
    pub state: Option<String>,
    pub title: Option<String>,
    pub unset_due_date: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
}

/// EditLabelOption options for editing a label
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditLabelOption {
    pub color: Option<String>,
    pub description: Option<String>,
    pub exclusive: Option<bool>,
    pub is_archived: Option<bool>,
    pub name: Option<String>,
}

/// EditMilestoneOption options for editing a milestone
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditMilestoneOption {
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_on: Option<time::OffsetDateTime>,
    pub state: Option<String>,
    pub title: Option<String>,
}

/// EditOrgOption options for editing an organization
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditOrgOption {
    pub description: Option<String>,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub location: Option<String>,
    pub repo_admin_change_team_access: Option<bool>,
    /// possible values are `public`, `limited` or `private`
    pub visibility: Option<EditOrgOptionVisibility>,
    pub website: Option<String>,
}

/// possible values are `public`, `limited` or `private`

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EditOrgOptionVisibility {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "limited")]
    Limited,
    #[serde(rename = "private")]
    Private,
}
/// EditPullRequestOption options when modify pull request
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditPullRequestOption {
    pub allow_maintainer_edit: Option<bool>,
    pub assignee: Option<String>,
    pub assignees: Option<Vec<String>>,
    pub base: Option<String>,
    pub body: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_date: Option<time::OffsetDateTime>,
    pub labels: Option<Vec<i64>>,
    pub milestone: Option<i64>,
    pub state: Option<String>,
    pub title: Option<String>,
    pub unset_due_date: Option<bool>,
}

/// EditQuotaRuleOptions represents the options for editing a quota rule
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditQuotaRuleOptions {
    /// The limit set by the rule
    pub limit: Option<i64>,
    /// The subjects affected by the rule
    pub subjects: Option<Vec<String>>,
}

/// EditReactionOption contain the reaction type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditReactionOption {
    pub content: Option<String>,
}

/// EditReleaseOption options when editing a release
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditReleaseOption {
    pub body: Option<String>,
    pub draft: Option<bool>,
    pub hide_archive_links: Option<bool>,
    pub name: Option<String>,
    pub prerelease: Option<bool>,
    pub tag_name: Option<String>,
    pub target_commitish: Option<String>,
}

/// EditRepoOption options when editing a repository's properties
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditRepoOption {
    /// either `true` to allow fast-forward-only merging pull requests, or `false` to prevent fast-forward-only merging.
    pub allow_fast_forward_only_merge: Option<bool>,
    /// either `true` to allow mark pr as merged manually, or `false` to prevent it.
    pub allow_manual_merge: Option<bool>,
    /// either `true` to allow merging pull requests with a merge commit, or `false` to prevent merging pull requests with merge commits.
    pub allow_merge_commits: Option<bool>,
    /// either `true` to allow rebase-merging pull requests, or `false` to prevent rebase-merging.
    pub allow_rebase: Option<bool>,
    /// either `true` to allow rebase with explicit merge commits (--no-ff), or `false` to prevent rebase with explicit merge commits.
    pub allow_rebase_explicit: Option<bool>,
    /// either `true` to allow updating pull request branch by rebase, or `false` to prevent it.
    pub allow_rebase_update: Option<bool>,
    /// either `true` to allow squash-merging pull requests, or `false` to prevent squash-merging.
    pub allow_squash_merge: Option<bool>,
    /// set to `true` to archive this repository.
    pub archived: Option<bool>,
    /// either `true` to enable AutodetectManualMerge, or `false` to prevent it. Note: In some special cases, misjudgments can occur.
    pub autodetect_manual_merge: Option<bool>,
    /// set to `true` to allow edits from maintainers by default
    pub default_allow_maintainer_edit: Option<bool>,
    /// sets the default branch for this repository.
    pub default_branch: Option<String>,
    /// set to `true` to delete pr branch after merge by default
    pub default_delete_branch_after_merge: Option<bool>,
    pub default_merge_style: Option<DefaultMergeStyle>,
    /// set to a update style to be used by this repository: "rebase" or "merge"
    pub default_update_style: Option<String>,
    /// a short description of the repository.
    pub description: Option<String>,
    /// enable prune - remove obsolete remote-tracking references when mirroring
    pub enable_prune: Option<bool>,
    pub external_tracker: Option<ExternalTracker>,
    pub external_wiki: Option<ExternalWiki>,
    /// set the globally editable state of the wiki
    pub globally_editable_wiki: Option<bool>,
    /// either `true` to enable actions unit, or `false` to disable them.
    pub has_actions: Option<bool>,
    /// either `true` to enable issues for this repository or `false` to disable them.
    pub has_issues: Option<bool>,
    /// either `true` to enable packages unit, or `false` to disable them.
    pub has_packages: Option<bool>,
    /// either `true` to enable project unit, or `false` to disable them.
    pub has_projects: Option<bool>,
    /// either `true` to allow pull requests, or `false` to prevent pull request.
    pub has_pull_requests: Option<bool>,
    /// either `true` to enable releases unit, or `false` to disable them.
    pub has_releases: Option<bool>,
    /// either `true` to enable the wiki for this repository or `false` to disable it.
    pub has_wiki: Option<bool>,
    /// either `true` to ignore whitespace for conflicts, or `false` to not ignore whitespace.
    pub ignore_whitespace_conflicts: Option<bool>,
    pub internal_tracker: Option<InternalTracker>,
    /// set to a string like `8h30m0s` to set the mirror interval time
    pub mirror_interval: Option<String>,
    /// name of the repository
    pub name: Option<String>,
    /// either `true` to make the repository private or `false` to make it public.
    ///
    /// Note: you will get a 422 error if the organization restricts changing repository visibility to organization
    ///
    /// owners and a non-owner tries to change the value of private.
    pub private: Option<bool>,
    /// either `true` to make this repository a template or `false` to make it a normal repository
    pub template: Option<bool>,
    /// a URL with more information about the repository.
    pub website: Option<String>,
    /// sets the branch used for this repository's wiki.
    pub wiki_branch: Option<String>,
}

/// EditTagProtectionOption options for editing a tag protection
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditTagProtectionOption {
    pub name_pattern: Option<String>,
    pub whitelist_teams: Option<Vec<String>>,
    pub whitelist_usernames: Option<Vec<String>>,
}

/// EditTeamOption options for editing a team
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditTeamOption {
    pub can_create_org_repo: Option<bool>,
    pub description: Option<String>,
    pub includes_all_repositories: Option<bool>,
    pub name: String,
    pub permission: Option<EditTeamOptionPermission>,
    pub units: Option<Vec<String>>,
    pub units_map: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EditTeamOptionPermission {
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "admin")]
    Admin,
}
/// EditUserOption edit user options
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditUserOption {
    pub active: Option<bool>,
    pub admin: Option<bool>,
    pub allow_create_organization: Option<bool>,
    pub allow_git_hook: Option<bool>,
    pub allow_import_local: Option<bool>,
    pub description: Option<String>,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub hide_email: Option<bool>,
    pub location: Option<String>,
    pub login_name: Option<String>,
    pub max_repo_creation: Option<i64>,
    pub must_change_password: Option<bool>,
    pub password: Option<String>,
    pub prohibit_login: Option<bool>,
    pub pronouns: Option<String>,
    pub restricted: Option<bool>,
    pub source_id: Option<i64>,
    pub visibility: Option<String>,
    pub website: Option<String>,
}

/// Email an email address belonging to a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Email {
    pub email: Option<String>,
    pub primary: Option<bool>,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub verified: Option<bool>,
}

impl_from_response!(Email);

/// ExternalTracker represents settings for external tracker
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalTracker {
    /// External Issue Tracker URL Format. Use the placeholders {user}, {repo} and {index} for the username, repository name and issue index.
    pub external_tracker_format: Option<String>,
    /// External Issue Tracker issue regular expression
    pub external_tracker_regexp_pattern: Option<String>,
    /// External Issue Tracker Number Format, either `numeric`, `alphanumeric`, or `regexp`
    pub external_tracker_style: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// URL of external issue tracker.
    pub external_tracker_url: Option<url::Url>,
}

/// ExternalWiki represents setting for external wiki
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalWiki {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// URL of external wiki.
    pub external_wiki_url: Option<url::Url>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileCommitResponse {
    pub author: Option<CommitUser>,
    pub committer: Option<CommitUser>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub message: Option<String>,
    pub parents: Option<Vec<CommitMeta>>,
    pub sha: Option<String>,
    pub tree: Option<CommitMeta>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

/// FileDeleteResponse contains information about a repo's file that was deleted
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileDeleteResponse {
    pub commit: Option<FileCommitResponse>,
    pub content: Option<serde_json::Value>,
    pub verification: Option<PayloadCommitVerification>,
}

impl_from_response!(FileDeleteResponse);

/// FileLinksResponse contains the links for a repo's file
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileLinksResponse {
    pub git: Option<String>,
    pub html: Option<String>,
    #[serde(rename = "self")]
    pub this: Option<String>,
}

/// FileResponse contains information about a repo's file
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileResponse {
    pub commit: Option<FileCommitResponse>,
    pub content: Option<ContentsResponse>,
    pub verification: Option<PayloadCommitVerification>,
}

impl_from_response!(FileResponse);

/// FilesResponse contains information about multiple files from a repo
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FilesResponse {
    pub commit: Option<FileCommitResponse>,
    pub files: Option<Vec<ContentsResponse>>,
    pub verification: Option<PayloadCommitVerification>,
}

impl_from_response!(FilesResponse);

/// ForgeLike activity data type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ForgeLike {}

/// ActivityStream OrderedCollection of activities
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ForgeOutbox {}

impl_from_response!(ForgeOutbox);

/// GPGKey a user GPG key to sign commit and tag in repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GPGKey {
    pub can_certify: Option<bool>,
    pub can_encrypt_comms: Option<bool>,
    pub can_encrypt_storage: Option<bool>,
    pub can_sign: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub emails: Option<Vec<GPGKeyEmail>>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<time::OffsetDateTime>,
    pub id: Option<i64>,
    pub key_id: Option<String>,
    pub primary_key_id: Option<String>,
    pub public_key: Option<String>,
    pub subkeys: Option<Vec<GPGKey>>,
    pub verified: Option<bool>,
}

impl_from_response!(GPGKey);

/// GPGKeyEmail an email attached to a GPGKey
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GPGKeyEmail {
    pub email: Option<String>,
    pub verified: Option<bool>,
}

/// GeneralAPISettings contains global api settings exposed by it
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneralAPISettings {
    pub default_git_trees_per_page: Option<i64>,
    pub default_max_blob_size: Option<i64>,
    pub default_paging_num: Option<i64>,
    pub max_response_items: Option<i64>,
}

impl_from_response!(GeneralAPISettings);

/// GeneralAttachmentSettings contains global Attachment settings exposed by API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneralAttachmentSettings {
    pub allowed_types: Option<String>,
    pub enabled: Option<bool>,
    pub max_files: Option<i64>,
    pub max_size: Option<i64>,
}

impl_from_response!(GeneralAttachmentSettings);

/// GeneralRepoSettings contains global repository settings exposed by API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneralRepoSettings {
    pub forks_disabled: Option<bool>,
    pub http_git_disabled: Option<bool>,
    pub lfs_disabled: Option<bool>,
    pub migrations_disabled: Option<bool>,
    pub mirrors_disabled: Option<bool>,
    pub stars_disabled: Option<bool>,
    pub time_tracking_disabled: Option<bool>,
}

impl_from_response!(GeneralRepoSettings);

/// GeneralUISettings contains global ui settings exposed by API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneralUISettings {
    pub allowed_reactions: Option<Vec<String>>,
    pub custom_emojis: Option<Vec<String>>,
    pub default_theme: Option<String>,
}

impl_from_response!(GeneralUISettings);

/// GenerateRepoOption options when creating repository using a template
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GenerateRepoOption {
    /// include avatar of the template repo
    pub avatar: Option<bool>,
    /// Default branch of the new repository
    pub default_branch: Option<String>,
    /// Description of the repository to create
    pub description: Option<String>,
    /// include git content of default branch in template repo
    pub git_content: Option<bool>,
    /// include git hooks in template repo
    pub git_hooks: Option<bool>,
    /// include labels in template repo
    pub labels: Option<bool>,
    /// Name of the repository to create
    pub name: String,
    /// The organization or person who will own the new repository
    pub owner: String,
    /// Whether the repository is private
    pub private: Option<bool>,
    /// include protected branches in template repo
    pub protected_branch: Option<bool>,
    /// include topics in template repo
    pub topics: Option<bool>,
    /// include webhooks in template repo
    pub webhooks: Option<bool>,
}

/// GitBlob represents a git blob
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GitBlob {
    pub content: Option<String>,
    pub encoding: Option<String>,
    pub sha: Option<String>,
    pub size: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(GitBlob);

/// GitEntry represents a git tree
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GitEntry {
    pub mode: Option<String>,
    pub path: Option<String>,
    pub sha: Option<String>,
    pub size: Option<i64>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

/// GitHook represents a Git repository hook
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GitHook {
    pub content: Option<String>,
    pub is_active: Option<bool>,
    pub name: Option<String>,
}

impl_from_response!(GitHook);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GitObject {
    pub sha: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

/// GitTreeResponse returns a git tree
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GitTreeResponse {
    pub page: Option<i64>,
    pub sha: Option<String>,
    pub total_count: Option<i64>,
    pub tree: Option<Vec<GitEntry>>,
    pub truncated: Option<bool>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(GitTreeResponse);

/// GitignoreTemplateInfo name and text of a gitignore template
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GitignoreTemplateInfo {
    pub name: Option<String>,
    pub source: Option<String>,
}

impl_from_response!(GitignoreTemplateInfo);

/// Hook a hook is a web hook when one repository changed
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Hook {
    pub active: Option<bool>,
    pub authorization_header: Option<String>,
    pub branch_filter: Option<String>,
    /// Deprecated: use Metadata instead
    pub config: Option<BTreeMap<String, String>>,
    pub content_type: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub events: Option<Vec<String>>,
    pub id: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(Hook);

/// Identity for a person's identity like an author or committer
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Identity {
    pub email: Option<String>,
    pub name: Option<String>,
}

/// InternalTracker represents settings for internal tracker
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InternalTracker {
    /// Let only contributors track time (Built-in issue tracker)
    pub allow_only_contributors_to_track_time: Option<bool>,
    /// Enable dependencies for issues and pull requests (Built-in issue tracker)
    pub enable_issue_dependencies: Option<bool>,
    /// Enable time tracking (Built-in issue tracker)
    pub enable_time_tracker: Option<bool>,
}

/// Issue represents an issue in a repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Issue {
    pub assets: Option<Vec<Attachment>>,
    pub assignee: Option<User>,
    pub assignees: Option<Vec<User>>,
    pub body: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub closed_at: Option<time::OffsetDateTime>,
    pub comments: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_date: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    pub is_locked: Option<bool>,
    pub labels: Option<Vec<Label>>,
    pub milestone: Option<Milestone>,
    pub number: Option<i64>,
    pub original_author: Option<String>,
    pub original_author_id: Option<i64>,
    pub pin_order: Option<i64>,
    pub pull_request: Option<PullRequestMeta>,
    #[serde(rename = "ref")]
    pub r#ref: Option<String>,
    pub repository: Option<RepositoryMeta>,
    pub state: Option<StateType>,
    pub title: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    pub user: Option<User>,
}

impl_from_response!(Issue);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssueConfig {
    pub blank_issues_enabled: Option<bool>,
    pub contact_links: Option<Vec<IssueConfigContactLink>>,
}

impl_from_response!(IssueConfig);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssueConfigContactLink {
    pub about: Option<String>,
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssueConfigValidation {
    pub message: Option<String>,
    pub valid: Option<bool>,
}

impl_from_response!(IssueConfigValidation);

/// IssueDeadline represents an issue deadline
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssueDeadline {
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_date: Option<time::OffsetDateTime>,
}

impl_from_response!(IssueDeadline);

/// IssueFormField represents a form field
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssueFormField {
    pub attributes: Option<BTreeMap<String, serde_json::Value>>,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<IssueFormFieldType>,
    pub validations: Option<BTreeMap<String, serde_json::Value>>,
    pub visible: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueFormFieldType {
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "textarea")]
    Textarea,
    #[serde(rename = "input")]
    Input,
    #[serde(rename = "dropdown")]
    Dropdown,
    #[serde(rename = "checkboxes")]
    Checkboxes,
}
/// IssueLabelsOption a collection of labels
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssueLabelsOption {
    /// Labels can be a list of integers representing label IDs
    ///
    /// or a list of strings representing label names
    pub labels: Option<Vec<serde_json::Value>>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
}

/// IssueMeta basic issue information
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssueMeta {
    pub index: Option<i64>,
    pub owner: Option<String>,
    pub repo: Option<String>,
}

/// IssueTemplate represents an issue template for a repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssueTemplate {
    pub about: Option<String>,
    pub body: Option<Vec<IssueFormField>>,
    pub content: Option<String>,
    pub file_name: Option<String>,
    pub labels: Option<Vec<String>>,
    pub name: Option<String>,
    #[serde(rename = "ref")]
    pub r#ref: Option<String>,
    pub title: Option<String>,
}

impl_from_response!(IssueTemplate);

/// Label a label to an issue or a pr
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Label {
    pub color: Option<String>,
    pub description: Option<String>,
    pub exclusive: Option<bool>,
    pub id: Option<i64>,
    pub is_archived: Option<bool>,
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(Label);

/// LabelTemplate info of a Label template
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LabelTemplate {
    pub color: Option<String>,
    pub description: Option<String>,
    pub exclusive: Option<bool>,
    pub name: Option<String>,
}

impl_from_response!(LabelTemplate);

/// LicensesInfo contains information about a License
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LicenseTemplateInfo {
    pub body: Option<String>,
    pub implementation: Option<String>,
    pub key: Option<String>,
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(LicenseTemplateInfo);

/// LicensesListEntry is used for the API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LicensesTemplateListEntry {
    pub key: Option<String>,
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(LicensesTemplateListEntry);

/// ListActionRunResponse return a list of ActionRun
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ListActionRunResponse {
    pub total_count: Option<i64>,
    pub workflow_runs: Option<Vec<ActionRun>>,
}

impl_from_response!(ListActionRunResponse);

/// MarkdownOption markdown options
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MarkdownOption {
    /// Context to render
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "Context")]
    pub context: Option<String>,
    /// Mode to render (comment, gfm, markdown)
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "Mode")]
    pub mode: Option<String>,
    /// Text markdown to render
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "Text")]
    pub text: Option<String>,
    /// Is it a wiki page ?
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "Wiki")]
    pub wiki: Option<bool>,
}

/// MarkupOption markup options
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MarkupOption {
    /// The current branch path where the form gets posted
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "BranchPath")]
    pub branch_path: Option<String>,
    /// Context to render
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "Context")]
    pub context: Option<String>,
    /// File path for detecting extension in file mode
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "FilePath")]
    pub file_path: Option<String>,
    /// Mode to render (comment, gfm, markdown, file)
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "Mode")]
    pub mode: Option<String>,
    /// Text markup to render
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "Text")]
    pub text: Option<String>,
    /// Is it a wiki page ?
    ///
    ///
    ///
    /// in: body
    #[serde(rename = "Wiki")]
    pub wiki: Option<bool>,
}

/// MergePullRequestForm form for merging Pull Request
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MergePullRequestOption {
    #[serde(rename = "Do")]
    pub r#do: MergePullRequestOptionDo,
    #[serde(rename = "MergeCommitID")]
    pub merge_commit_id: Option<String>,
    #[serde(rename = "MergeMessageField")]
    pub merge_message_field: Option<String>,
    #[serde(rename = "MergeTitleField")]
    pub merge_title_field: Option<String>,
    pub delete_branch_after_merge: Option<bool>,
    pub force_merge: Option<bool>,
    pub head_commit_id: Option<String>,
    pub merge_when_checks_succeed: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MergePullRequestOptionDo {
    #[serde(rename = "merge")]
    Merge,
    #[serde(rename = "rebase")]
    Rebase,
    #[serde(rename = "rebase-merge")]
    RebaseMerge,
    #[serde(rename = "squash")]
    Squash,
    #[serde(rename = "fast-forward-only")]
    FastForwardOnly,
    #[serde(rename = "manually-merged")]
    ManuallyMerged,
}
/// MigrateRepoOptions options for migrating repository's
///
/// this is used to interact with api v1
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MigrateRepoOptions {
    pub auth_password: Option<String>,
    pub auth_token: Option<String>,
    pub auth_username: Option<String>,
    pub clone_addr: String,
    pub description: Option<String>,
    pub issues: Option<bool>,
    pub labels: Option<bool>,
    pub lfs: Option<bool>,
    pub lfs_endpoint: Option<String>,
    pub milestones: Option<bool>,
    pub mirror: Option<bool>,
    pub mirror_interval: Option<String>,
    pub private: Option<bool>,
    pub pull_requests: Option<bool>,
    pub releases: Option<bool>,
    pub repo_name: String,
    /// Name of User or Organisation who will own Repo after migration
    pub repo_owner: Option<String>,
    pub service: Option<MigrateRepoOptionsService>,
    /// deprecated (only for backwards compatibility)
    pub uid: Option<i64>,
    pub wiki: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MigrateRepoOptionsService {
    #[serde(rename = "git")]
    Git,
    #[serde(rename = "github")]
    Github,
    #[serde(rename = "gitea")]
    Gitea,
    #[serde(rename = "gitlab")]
    Gitlab,
    #[serde(rename = "gogs")]
    Gogs,
    #[serde(rename = "onedev")]
    Onedev,
    #[serde(rename = "gitbucket")]
    Gitbucket,
    #[serde(rename = "codebase")]
    Codebase,
}
/// Milestone milestone is a collection of issues on one repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Milestone {
    #[serde(with = "time::serde::rfc3339::option")]
    pub closed_at: Option<time::OffsetDateTime>,
    pub closed_issues: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_on: Option<time::OffsetDateTime>,
    pub id: Option<i64>,
    pub open_issues: Option<i64>,
    pub state: Option<StateType>,
    pub title: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
}

impl_from_response!(Milestone);

/// NewIssuePinsAllowed represents an API response that says if new Issue Pins are allowed
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NewIssuePinsAllowed {
    pub issues: Option<bool>,
    pub pull_requests: Option<bool>,
}

impl_from_response!(NewIssuePinsAllowed);

/// NodeInfo contains standardized way of exposing metadata about a server running one of the distributed social networks
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeInfo {
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(rename = "openRegistrations")]
    pub open_registrations: Option<bool>,
    pub protocols: Option<Vec<String>>,
    pub services: Option<NodeInfoServices>,
    pub software: Option<NodeInfoSoftware>,
    pub usage: Option<NodeInfoUsage>,
    pub version: Option<String>,
}

impl_from_response!(NodeInfo);

/// NodeInfoServices contains the third party sites this server can connect to via their application API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeInfoServices {
    pub inbound: Option<Vec<String>>,
    pub outbound: Option<Vec<String>>,
}

/// NodeInfoSoftware contains Metadata about server software in use
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeInfoSoftware {
    pub homepage: Option<String>,
    pub name: Option<String>,
    pub repository: Option<String>,
    pub version: Option<String>,
}

/// NodeInfoUsage contains usage statistics for this server
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeInfoUsage {
    #[serde(rename = "localComments")]
    pub local_comments: Option<i64>,
    #[serde(rename = "localPosts")]
    pub local_posts: Option<i64>,
    pub users: Option<NodeInfoUsageUsers>,
}

/// NodeInfoUsageUsers contains statistics about the users of this server
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeInfoUsageUsers {
    #[serde(rename = "activeHalfyear")]
    pub active_halfyear: Option<i64>,
    #[serde(rename = "activeMonth")]
    pub active_month: Option<i64>,
    pub total: Option<i64>,
}

/// Note contains information related to a git note
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub commit: Option<Commit>,
    pub message: Option<String>,
}

impl_from_response!(Note);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoteOptions {
    pub message: Option<String>,
}

/// NotificationCount number of unread notifications
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NotificationCount {
    pub new: Option<i64>,
}

impl_from_response!(NotificationCount);

/// NotificationSubject contains the notification subject (Issue/Pull/Commit)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NotificationSubject {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub latest_comment_html_url: Option<url::Url>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub latest_comment_url: Option<url::Url>,
    pub state: Option<StateType>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<NotifySubjectType>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

/// NotificationThread expose Notification on API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NotificationThread {
    pub id: Option<i64>,
    pub pinned: Option<bool>,
    pub repository: Option<Repository>,
    pub subject: Option<NotificationSubject>,
    pub unread: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(NotificationThread);

/// NotifySubjectType represent type of notification subject

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NotifySubjectType {
    #[serde(rename = "Issue")]
    Issue,
    #[serde(rename = "Pull")]
    Pull,
    #[serde(rename = "Commit")]
    Commit,
    #[serde(rename = "Repository")]
    Repository,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OAuth2Application {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub confidential_client: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
}

impl_from_response!(OAuth2Application);

/// ObjectFormatName of the underlying git repository

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObjectFormatName {
    #[serde(rename = "sha1")]
    Sha1,
    #[serde(rename = "sha256")]
    Sha256,
}
/// Organization represents an organization
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Organization {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub avatar_url: Option<url::Url>,
    pub description: Option<String>,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub id: Option<i64>,
    pub location: Option<String>,
    pub name: Option<String>,
    pub repo_admin_change_team_access: Option<bool>,
    /// deprecated
    pub username: Option<String>,
    pub visibility: Option<String>,
    pub website: Option<String>,
}

impl_from_response!(Organization);

/// OrganizationPermissions list different users permissions on an organization
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OrganizationPermissions {
    pub can_create_repository: Option<bool>,
    pub can_read: Option<bool>,
    pub can_write: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_owner: Option<bool>,
}

impl_from_response!(OrganizationPermissions);

/// PRBranchInfo information about a branch
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PRBranchInfo {
    pub label: Option<String>,
    #[serde(rename = "ref")]
    pub r#ref: Option<String>,
    pub repo: Option<Repository>,
    pub repo_id: Option<i64>,
    pub sha: Option<String>,
}

/// Package represents a package
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Package {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub creator: Option<User>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub owner: Option<User>,
    pub repository: Option<Repository>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub version: Option<String>,
}

impl_from_response!(Package);

/// PackageFile represents a package file
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PackageFile {
    #[serde(rename = "Size")]
    pub size: Option<i64>,
    pub id: Option<i64>,
    pub md5: Option<String>,
    pub name: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
}

impl_from_response!(PackageFile);

/// PayloadCommit represents a commit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayloadCommit {
    pub added: Option<Vec<String>>,
    pub author: Option<PayloadUser>,
    pub committer: Option<PayloadUser>,
    /// sha1 hash of the commit
    pub id: Option<String>,
    pub message: Option<String>,
    pub modified: Option<Vec<String>>,
    pub removed: Option<Vec<String>>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub timestamp: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    pub verification: Option<PayloadCommitVerification>,
}

/// PayloadCommitVerification represents the GPG verification of a commit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayloadCommitVerification {
    pub payload: Option<String>,
    pub reason: Option<String>,
    pub signature: Option<String>,
    pub signer: Option<PayloadUser>,
    pub verified: Option<bool>,
}

/// PayloadUser represents the author or committer of a commit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayloadUser {
    pub email: Option<String>,
    /// Full name of the commit author
    pub name: Option<String>,
    pub username: Option<String>,
}

/// Permission represents a set of permissions
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Permission {
    pub admin: Option<bool>,
    pub pull: Option<bool>,
    pub push: Option<bool>,
}

/// PublicKey publickey is a user key to push code to repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PublicKey {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub fingerprint: Option<String>,
    pub id: Option<i64>,
    pub key: Option<String>,
    pub key_type: Option<String>,
    pub read_only: Option<bool>,
    pub title: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    pub user: Option<User>,
}

impl_from_response!(PublicKey);

/// PullRequest represents a pull request
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PullRequest {
    pub additions: Option<i64>,
    pub allow_maintainer_edit: Option<bool>,
    pub assignee: Option<User>,
    pub assignees: Option<Vec<User>>,
    pub base: Option<PRBranchInfo>,
    pub body: Option<String>,
    pub changed_files: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub closed_at: Option<time::OffsetDateTime>,
    pub comments: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub deletions: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub diff_url: Option<url::Url>,
    pub draft: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_date: Option<time::OffsetDateTime>,
    pub flow: Option<i64>,
    pub head: Option<PRBranchInfo>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    pub is_locked: Option<bool>,
    pub labels: Option<Vec<Label>>,
    pub merge_base: Option<String>,
    pub merge_commit_sha: Option<String>,
    pub mergeable: Option<bool>,
    pub merged: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub merged_at: Option<time::OffsetDateTime>,
    pub merged_by: Option<User>,
    pub milestone: Option<Milestone>,
    pub number: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub patch_url: Option<url::Url>,
    pub pin_order: Option<i64>,
    #[serde(deserialize_with = "crate::requested_reviewers_ignore_null")]
    pub requested_reviewers: Option<Vec<User>>,
    pub requested_reviewers_teams: Option<Vec<Team>>,
    /// number of review comments made on the diff of a PR review (not including comments on commits or issues in a PR)
    pub review_comments: Option<i64>,
    pub state: Option<StateType>,
    pub title: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    pub user: Option<User>,
}

impl_from_response!(PullRequest);

/// PullRequestMeta PR info if an issue is a PR
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PullRequestMeta {
    pub draft: Option<bool>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub merged: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub merged_at: Option<time::OffsetDateTime>,
}

/// PullReview represents a pull request review
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PullReview {
    pub body: Option<String>,
    pub comments_count: Option<i64>,
    pub commit_id: Option<String>,
    pub dismissed: Option<bool>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    pub official: Option<bool>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub pull_request_url: Option<url::Url>,
    pub stale: Option<bool>,
    pub state: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<time::OffsetDateTime>,
    pub team: Option<Team>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    pub user: Option<User>,
}

impl_from_response!(PullReview);

/// PullReviewComment represents a comment on a pull request review
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PullReviewComment {
    pub body: Option<String>,
    pub commit_id: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub diff_hunk: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    pub original_commit_id: Option<String>,
    pub original_position: Option<u64>,
    pub path: Option<String>,
    pub position: Option<u64>,
    pub pull_request_review_id: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub pull_request_url: Option<url::Url>,
    pub resolver: Option<User>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    pub user: Option<User>,
}

impl_from_response!(PullReviewComment);

/// PullReviewRequestOptions are options to add or remove pull review requests
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PullReviewRequestOptions {
    pub reviewers: Option<Vec<String>>,
    pub team_reviewers: Option<Vec<String>>,
}

/// PushMirror represents information of a push mirror
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PushMirror {
    pub branch_filter: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    pub interval: Option<String>,
    pub last_error: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_update: Option<time::OffsetDateTime>,
    pub public_key: Option<String>,
    pub remote_address: Option<String>,
    pub remote_name: Option<String>,
    pub repo_name: Option<String>,
    pub sync_on_commit: Option<bool>,
}

impl_from_response!(PushMirror);

/// QuotaGroup represents a quota group
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaGroup {
    /// Name of the group
    pub name: Option<String>,
    /// Rules associated with the group
    pub rules: Option<Vec<QuotaRuleInfo>>,
}

impl_from_response!(QuotaGroup);

/// QuotaInfo represents information about a user's quota
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaInfo {
    pub groups: Option<Vec<QuotaGroup>>,
    pub used: Option<QuotaUsed>,
}

impl_from_response!(QuotaInfo);

/// QuotaRuleInfo contains information about a quota rule
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaRuleInfo {
    /// The limit set by the rule
    pub limit: Option<i64>,
    /// Name of the rule (only shown to admins)
    pub name: Option<String>,
    /// Subjects the rule affects
    pub subjects: Option<Vec<String>>,
}

impl_from_response!(QuotaRuleInfo);

/// QuotaUsed represents the quota usage of a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsed {
    pub size: Option<QuotaUsedSize>,
}

/// QuotaUsedArtifact represents an artifact counting towards a user's quota
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedArtifact {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// HTML URL to the action run containing the artifact
    pub html_url: Option<url::Url>,
    /// Name of the artifact
    pub name: Option<String>,
    /// Size of the artifact (compressed)
    pub size: Option<i64>,
}

impl_from_response!(QuotaUsedArtifact);

/// QuotaUsedAttachment represents an attachment counting towards a user's quota
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedAttachment {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// API URL for the attachment
    pub api_url: Option<url::Url>,
    /// Context for the attachment: URLs to the containing object
    pub contained_in: Option<QuotaUsedAttachmentContainedIn>,
    /// Filename of the attachment
    pub name: Option<String>,
    /// Size of the attachment (in bytes)
    pub size: Option<i64>,
}

impl_from_response!(QuotaUsedAttachment);

/// Context for the attachment: URLs to the containing object
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedAttachmentContainedIn {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// API URL for the object that contains this attachment
    pub api_url: Option<url::Url>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// HTML URL for the object that contains this attachment
    pub html_url: Option<url::Url>,
}

/// QuotaUsedPackage represents a package counting towards a user's quota
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedPackage {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// HTML URL to the package version
    pub html_url: Option<url::Url>,
    /// Name of the package
    pub name: Option<String>,
    /// Size of the package version
    pub size: Option<i64>,
    /// Type of the package
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    /// Version of the package
    pub version: Option<String>,
}

impl_from_response!(QuotaUsedPackage);

/// QuotaUsedSize represents the size-based quota usage of a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedSize {
    pub assets: Option<QuotaUsedSizeAssets>,
    pub git: Option<QuotaUsedSizeGit>,
    pub repos: Option<QuotaUsedSizeRepos>,
}

/// QuotaUsedSizeAssets represents the size-based asset usage of a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedSizeAssets {
    /// Storage size used for the user's artifacts
    pub artifacts: Option<i64>,
    pub attachments: Option<QuotaUsedSizeAssetsAttachments>,
    pub packages: Option<QuotaUsedSizeAssetsPackages>,
}

/// QuotaUsedSizeAssetsAttachments represents the size-based attachment quota usage of a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedSizeAssetsAttachments {
    /// Storage size used for the user's issue & comment attachments
    pub issues: Option<i64>,
    /// Storage size used for the user's release attachments
    pub releases: Option<i64>,
}

/// QuotaUsedSizeAssetsPackages represents the size-based package quota usage of a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedSizeAssetsPackages {
    /// Storage suze used for the user's packages
    pub all: Option<i64>,
}

/// QuotaUsedSizeGit represents the size-based git (lfs) quota usage of a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedSizeGit {
    /// Storage size of the user's Git LFS objects
    #[serde(rename = "LFS")]
    pub lfs: Option<i64>,
}

/// QuotaUsedSizeRepos represents the size-based repository quota usage of a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuotaUsedSizeRepos {
    /// Storage size of the user's private repositories
    pub private: Option<i64>,
    /// Storage size of the user's public repositories
    pub public: Option<i64>,
}

/// Reaction contain one reaction
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Reaction {
    pub content: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub user: Option<User>,
}

impl_from_response!(Reaction);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Reference {
    pub object: Option<GitObject>,
    #[serde(rename = "ref")]
    pub r#ref: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(Reference);

/// RegistrationToken is a string used to register a runner with a server
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RegistrationToken {
    pub token: Option<String>,
}

impl_from_response!(RegistrationToken);

/// Release represents a repository release
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Release {
    pub archive_download_count: Option<TagArchiveDownloadCount>,
    pub assets: Option<Vec<Attachment>>,
    pub author: Option<User>,
    pub body: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub draft: Option<bool>,
    pub hide_archive_links: Option<bool>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub prerelease: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<time::OffsetDateTime>,
    pub tag_name: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub tarball_url: Option<url::Url>,
    pub target_commitish: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub upload_url: Option<url::Url>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub zipball_url: Option<url::Url>,
}

impl_from_response!(Release);

/// RenameOrgOption options when renaming an organization
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RenameOrgOption {
    /// New username for this org. This name cannot be in use yet by any other user.
    pub new_name: String,
}

/// RenameUserOption options when renaming a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RenameUserOption {
    /// New username for this user. This name cannot be in use yet by any other user.
    pub new_username: String,
}

/// ReplaceFlagsOption options when replacing the flags of a repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReplaceFlagsOption {
    pub flags: Option<Vec<String>>,
}

/// RepoCollaboratorPermission to get repository permission for a collaborator
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RepoCollaboratorPermission {
    pub permission: Option<String>,
    pub role_name: Option<String>,
    pub user: Option<User>,
}

impl_from_response!(RepoCollaboratorPermission);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RepoCommit {
    pub author: Option<CommitUser>,
    pub committer: Option<CommitUser>,
    pub message: Option<String>,
    pub tree: Option<CommitMeta>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    pub verification: Option<PayloadCommitVerification>,
}

/// RepoTopicOptions a collection of repo topic names
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RepoTopicOptions {
    /// list of topic names
    pub topics: Option<Vec<String>>,
}

/// RepoTransfer represents a pending repo transfer
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RepoTransfer {
    pub doer: Option<User>,
    pub recipient: Option<User>,
    pub teams: Option<Vec<Team>>,
}

/// Repository represents a repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Repository {
    pub allow_fast_forward_only_merge: Option<bool>,
    pub allow_merge_commits: Option<bool>,
    pub allow_rebase: Option<bool>,
    pub allow_rebase_explicit: Option<bool>,
    pub allow_rebase_update: Option<bool>,
    pub allow_squash_merge: Option<bool>,
    pub archived: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub archived_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub avatar_url: Option<url::Url>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub clone_url: Option<url::Url>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub default_allow_maintainer_edit: Option<bool>,
    pub default_branch: Option<String>,
    pub default_delete_branch_after_merge: Option<bool>,
    pub default_merge_style: Option<DefaultMergeStyle>,
    pub default_update_style: Option<String>,
    pub description: Option<String>,
    pub empty: Option<bool>,
    pub external_tracker: Option<ExternalTracker>,
    pub external_wiki: Option<ExternalWiki>,
    pub fork: Option<bool>,
    pub forks_count: Option<i64>,
    pub full_name: Option<String>,
    pub globally_editable_wiki: Option<bool>,
    pub has_actions: Option<bool>,
    pub has_issues: Option<bool>,
    pub has_packages: Option<bool>,
    pub has_projects: Option<bool>,
    pub has_pull_requests: Option<bool>,
    pub has_releases: Option<bool>,
    pub has_wiki: Option<bool>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    pub ignore_whitespace_conflicts: Option<bool>,
    pub internal: Option<bool>,
    pub internal_tracker: Option<InternalTracker>,
    pub language: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub languages_url: Option<url::Url>,
    pub link: Option<String>,
    pub mirror: Option<bool>,
    pub mirror_interval: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub mirror_updated: Option<time::OffsetDateTime>,
    pub name: Option<String>,
    pub object_format_name: Option<ObjectFormatName>,
    pub open_issues_count: Option<i64>,
    pub open_pr_counter: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub original_url: Option<url::Url>,
    pub owner: Option<User>,
    pub parent: Option<Box<Repository>>,
    pub permissions: Option<Permission>,
    pub private: Option<bool>,
    pub release_counter: Option<i64>,
    pub repo_transfer: Option<RepoTransfer>,
    pub size: Option<i64>,
    #[serde(deserialize_with = "crate::deserialize_optional_ssh_url")]
    pub ssh_url: Option<url::Url>,
    pub stars_count: Option<i64>,
    pub template: Option<bool>,
    pub topics: Option<Vec<String>>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
    pub watchers_count: Option<i64>,
    pub website: Option<String>,
    pub wiki_branch: Option<String>,
}

impl_from_response!(Repository);

/// RepositoryMeta basic repository information
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RepositoryMeta {
    pub full_name: Option<String>,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub owner: Option<String>,
}

/// SearchResults results of a successful search
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchResults {
    pub data: Option<Vec<Repository>>,
    pub ok: Option<bool>,
}

impl_from_response!(SearchResults);

/// Secret represents a secret
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Secret {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    /// the secret's name
    pub name: Option<String>,
}

impl_from_response!(Secret);

/// ServerVersion wraps the version of the server
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServerVersion {
    pub version: Option<String>,
}

impl_from_response!(ServerVersion);

/// SetUserQuotaGroupsOptions represents the quota groups of a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SetUserQuotaGroupsOptions {
    /// Quota groups the user shall have
    pub groups: Vec<String>,
}

/// StateType issue state type

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StateType {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
}
/// StopWatch represent a running stopwatch
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StopWatch {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    pub duration: Option<String>,
    pub issue_index: Option<i64>,
    pub issue_title: Option<String>,
    pub repo_name: Option<String>,
    pub repo_owner_name: Option<String>,
    pub seconds: Option<i64>,
}

impl_from_response!(StopWatch);

/// SubmitPullReviewOptions are options to submit a pending pull review
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubmitPullReviewOptions {
    pub body: Option<String>,
    pub event: Option<String>,
}

/// SyncForkInfo information about syncing a fork
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SyncForkInfo {
    pub allowed: Option<bool>,
    pub base_commit: Option<String>,
    pub commits_behind: Option<i64>,
    pub fork_commit: Option<String>,
}

impl_from_response!(SyncForkInfo);

/// Tag represents a repository tag
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tag {
    pub archive_download_count: Option<TagArchiveDownloadCount>,
    pub commit: Option<CommitMeta>,
    pub id: Option<String>,
    pub message: Option<String>,
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub tarball_url: Option<url::Url>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub zipball_url: Option<url::Url>,
}

impl_from_response!(Tag);

/// TagArchiveDownloadCount counts how many times a archive was downloaded
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TagArchiveDownloadCount {
    pub tar_gz: Option<i64>,
    pub zip: Option<i64>,
}

/// TagProtection represents a tag protection
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TagProtection {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub id: Option<i64>,
    pub name_pattern: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    pub whitelist_teams: Option<Vec<String>>,
    pub whitelist_usernames: Option<Vec<String>>,
}

impl_from_response!(TagProtection);

/// Team represents a team in an organization
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Team {
    pub can_create_org_repo: Option<bool>,
    pub description: Option<String>,
    pub id: Option<i64>,
    pub includes_all_repositories: Option<bool>,
    pub name: Option<String>,
    pub organization: Option<Organization>,
    pub permission: Option<TeamPermission>,
    pub units: Option<Vec<String>>,
    pub units_map: Option<BTreeMap<String, String>>,
}

impl_from_response!(Team);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TeamPermission {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "owner")]
    Owner,
}
/// TimelineComment represents a timeline comment (comment of any type) on a commit or issue
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimelineComment {
    pub assignee: Option<User>,
    pub assignee_team: Option<Team>,
    pub body: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub dependent_issue: Option<Issue>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub id: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub issue_url: Option<url::Url>,
    pub label: Option<Label>,
    pub milestone: Option<Milestone>,
    pub new_ref: Option<String>,
    pub new_title: Option<String>,
    pub old_milestone: Option<Milestone>,
    pub old_project_id: Option<i64>,
    pub old_ref: Option<String>,
    pub old_title: Option<String>,
    pub project_id: Option<i64>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub pull_request_url: Option<url::Url>,
    pub ref_action: Option<String>,
    pub ref_comment: Option<Comment>,
    /// commit SHA where issue/PR was referenced
    pub ref_commit_sha: Option<String>,
    pub ref_issue: Option<Issue>,
    /// whether the assignees were removed or added
    pub removed_assignee: Option<bool>,
    pub resolve_doer: Option<User>,
    pub review_id: Option<i64>,
    pub tracked_time: Option<TrackedTime>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<time::OffsetDateTime>,
    pub user: Option<User>,
}

impl_from_response!(TimelineComment);

/// TopicName a list of repo topic names
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TopicName {
    pub topics: Option<Vec<String>>,
}

impl_from_response!(TopicName);

/// TopicResponse for returning topics
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TopicResponse {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    pub id: Option<i64>,
    pub repo_count: Option<i64>,
    pub topic_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated: Option<time::OffsetDateTime>,
}

/// TrackedTime worked time for an issue / pr
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrackedTime {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    pub id: Option<i64>,
    pub issue: Option<Issue>,
    /// deprecated (only for backwards compatibility)
    pub issue_id: Option<i64>,
    /// Time in seconds
    pub time: Option<i64>,
    /// deprecated (only for backwards compatibility)
    pub user_id: Option<i64>,
    pub user_name: Option<String>,
}

impl_from_response!(TrackedTime);

/// TransferRepoOption options when transfer a repository's ownership
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransferRepoOption {
    pub new_owner: String,
    /// ID of the team or teams to add to the repository. Teams can only be added to organization-owned repositories.
    pub team_ids: Option<Vec<i64>>,
}

/// UpdateBranchRepoOption options when updating a branch in a repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UpdateBranchRepoOption {
    /// New branch name
    pub name: String,
}

/// UpdateFileOptions options for updating files
///
/// Note: `author` and `committer` are optional (if only one is given, it will be used for the other, otherwise the authenticated user will be used)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UpdateFileOptions {
    pub author: Option<Identity>,
    /// branch (optional) to base this file from. if not given, the default branch is used
    pub branch: Option<String>,
    pub committer: Option<Identity>,
    /// content must be base64 encoded
    pub content: String,
    pub dates: Option<CommitDateOptions>,
    /// from_path (optional) is the path of the original file which will be moved/renamed to the path in the URL
    pub from_path: Option<String>,
    /// message (optional) for the commit of this file. if not supplied, a default message will be used
    pub message: Option<String>,
    /// new_branch (optional) will make a new branch from `branch` before creating the file
    pub new_branch: Option<String>,
    /// sha is the SHA for the file that already exists
    pub sha: String,
    /// Add a Signed-off-by trailer by the committer at the end of the commit log message.
    pub signoff: Option<bool>,
}

/// UpdateRepoAvatarUserOption options when updating the repo avatar
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UpdateRepoAvatarOption {
    /// image must be base64 encoded
    pub image: Option<String>,
}

/// UpdateUserAvatarUserOption options when updating the user avatar
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UpdateUserAvatarOption {
    /// image must be base64 encoded
    pub image: Option<String>,
}

/// UpdateVariableOption the option when updating variable
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UpdateVariableOption {
    /// New name for the variable. If the field is empty, the variable name won't be updated.
    pub name: Option<String>,
    /// Value of the variable to update
    pub value: String,
}

/// User represents a user
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct User {
    /// Is user active
    pub active: Option<bool>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// URL to the user's avatar
    pub avatar_url: Option<url::Url>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub created: Option<time::OffsetDateTime>,
    /// the user's description
    pub description: Option<String>,
    pub email: Option<String>,
    /// user counts
    pub followers_count: Option<i64>,
    pub following_count: Option<i64>,
    /// the user's full name
    pub full_name: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    /// URL to the user's profile page
    pub html_url: Option<url::Url>,
    /// the user's id
    pub id: Option<i64>,
    /// Is the user an administrator
    pub is_admin: Option<bool>,
    /// User locale
    pub language: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_login: Option<time::OffsetDateTime>,
    /// the user's location
    pub location: Option<String>,
    /// the user's username
    pub login: Option<String>,
    /// the user's authentication sign-in name.
    pub login_name: Option<String>,
    /// Is user login prohibited
    pub prohibit_login: Option<bool>,
    /// the user's pronouns
    pub pronouns: Option<String>,
    /// Is user restricted
    pub restricted: Option<bool>,
    /// The ID of the user's Authentication Source
    pub source_id: Option<i64>,
    pub starred_repos_count: Option<i64>,
    /// User visibility level option: public, limited, private
    pub visibility: Option<String>,
    /// the user's website
    pub website: Option<String>,
}

impl_from_response!(User);

/// UserHeatmapData represents the data needed to create a heatmap
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserHeatmapData {
    pub contributions: Option<i64>,
    pub timestamp: Option<i64>,
}

impl_from_response!(UserHeatmapData);

/// UserSettings represents user settings
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserSettings {
    pub description: Option<String>,
    pub diff_view_style: Option<String>,
    pub enable_repo_unit_hints: Option<bool>,
    pub full_name: Option<String>,
    pub hide_activity: Option<bool>,
    /// Privacy
    pub hide_email: Option<bool>,
    pub hide_pronouns: Option<bool>,
    pub language: Option<String>,
    pub location: Option<String>,
    pub pronouns: Option<String>,
    pub theme: Option<String>,
    pub website: Option<String>,
}

impl_from_response!(UserSettings);

/// UserSettingsOptions represents options to change user settings
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserSettingsOptions {
    pub description: Option<String>,
    pub diff_view_style: Option<String>,
    pub enable_repo_unit_hints: Option<bool>,
    pub full_name: Option<String>,
    pub hide_activity: Option<bool>,
    /// Privacy
    pub hide_email: Option<bool>,
    pub hide_pronouns: Option<bool>,
    pub language: Option<String>,
    pub location: Option<String>,
    pub pronouns: Option<String>,
    pub theme: Option<String>,
    pub website: Option<String>,
}

/// VerifyGPGKeyOption options verifies user GPG key
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VerifyGPGKeyOption {
    pub armored_signature: Option<String>,
    /// An Signature for a GPG key token
    pub key_id: String,
}

/// WatchInfo represents an API watch status of one repository
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WatchInfo {
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<time::OffsetDateTime>,
    pub ignored: Option<bool>,
    pub reason: Option<serde_json::Value>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub repository_url: Option<url::Url>,
    pub subscribed: Option<bool>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub url: Option<url::Url>,
}

impl_from_response!(WatchInfo);

/// WikiCommit page commit/revision
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WikiCommit {
    pub author: Option<CommitUser>,
    pub commiter: Option<CommitUser>,
    pub message: Option<String>,
    pub sha: Option<String>,
}

/// WikiCommitList commit/revision list
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WikiCommitList {
    pub commits: Option<Vec<WikiCommit>>,
    pub count: Option<i64>,
}

impl_from_response!(WikiCommitList);

/// WikiPage a wiki page
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WikiPage {
    pub commit_count: Option<i64>,
    /// Page content, base64 encoded
    pub content_base64: Option<String>,
    pub footer: Option<String>,
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub last_commit: Option<WikiCommit>,
    pub sidebar: Option<String>,
    pub sub_url: Option<String>,
    pub title: Option<String>,
}

impl_from_response!(WikiPage);

/// WikiPageMetaData wiki page meta information
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WikiPageMetaData {
    #[serde(deserialize_with = "crate::none_if_blank_url")]
    pub html_url: Option<url::Url>,
    pub last_commit: Option<WikiCommit>,
    pub sub_url: Option<String>,
    pub title: Option<String>,
}

impl_from_response!(WikiPageMetaData);

pub struct AccessTokenListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for AccessTokenListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for AccessTokenListHeaders {}
impl crate::CountHeader for AccessTokenListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct ActivityFeedsListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for ActivityFeedsListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for ActivityFeedsListHeaders {}
impl crate::CountHeader for ActivityFeedsListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct BlockedUserListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for BlockedUserListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for BlockedUserListHeaders {}
impl crate::CountHeader for BlockedUserListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct BranchListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for BranchListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for BranchListHeaders {}
impl crate::CountHeader for BranchListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct ChangedFileListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for ChangedFileListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for ChangedFileListHeaders {}
impl crate::CountHeader for ChangedFileListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct ChangedFileListWithPaginationHeaders {
    pub x_has_more: Option<bool>,
    pub x_page: Option<i64>,
    pub x_page_count: Option<i64>,
    pub x_per_page: Option<i64>,
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for ChangedFileListWithPaginationHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_has_more = map
            .get("x-hasmore")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<bool>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        let x_page = map
            .get("x-page")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        let x_page_count = map
            .get("x-pagecount")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        let x_per_page = map
            .get("x-perpage")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self {
            x_has_more,
            x_page,
            x_page_count,
            x_per_page,
            x_total_count,
        })
    }
}

impl crate::sealed::Sealed for ChangedFileListWithPaginationHeaders {}
impl crate::CountHeader for ChangedFileListWithPaginationHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct CombinedStatusHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for CombinedStatusHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for CombinedStatusHeaders {}
impl crate::CountHeader for CombinedStatusHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct CommentListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for CommentListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for CommentListHeaders {}
impl crate::CountHeader for CommentListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct CommitListHeaders {
    pub x_has_more: Option<bool>,
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for CommitListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_has_more = map
            .get("x-hasmore")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<bool>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self {
            x_has_more,
            x_total_count,
        })
    }
}

impl crate::sealed::Sealed for CommitListHeaders {}
impl crate::CountHeader for CommitListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct CommitStatusListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for CommitStatusListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for CommitStatusListHeaders {}
impl crate::CountHeader for CommitStatusListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct CronListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for CronListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for CronListHeaders {}
impl crate::CountHeader for CronListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct DeployKeyListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for DeployKeyListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for DeployKeyListHeaders {}
impl crate::CountHeader for DeployKeyListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct GpgKeyListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for GpgKeyListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for GpgKeyListHeaders {}
impl crate::CountHeader for GpgKeyListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct HookListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for HookListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for HookListHeaders {}
impl crate::CountHeader for HookListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct IssueListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for IssueListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for IssueListHeaders {}
impl crate::CountHeader for IssueListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct LabelListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for LabelListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for LabelListHeaders {}
impl crate::CountHeader for LabelListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct MilestoneListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for MilestoneListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for MilestoneListHeaders {}
impl crate::CountHeader for MilestoneListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct NotificationThreadListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for NotificationThreadListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for NotificationThreadListHeaders {}
impl crate::CountHeader for NotificationThreadListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct OAuth2ApplicationListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for OAuth2ApplicationListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for OAuth2ApplicationListHeaders {}
impl crate::CountHeader for OAuth2ApplicationListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct OrganizationListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for OrganizationListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for OrganizationListHeaders {}
impl crate::CountHeader for OrganizationListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct PackageListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for PackageListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for PackageListHeaders {}
impl crate::CountHeader for PackageListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct PublicKeyListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for PublicKeyListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for PublicKeyListHeaders {}
impl crate::CountHeader for PublicKeyListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct PullRequestListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for PullRequestListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for PullRequestListHeaders {}
impl crate::CountHeader for PullRequestListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct PullReviewListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for PullReviewListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for PullReviewListHeaders {}
impl crate::CountHeader for PullReviewListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct PushMirrorListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for PushMirrorListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for PushMirrorListHeaders {}
impl crate::CountHeader for PushMirrorListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct QuotaUsedArtifactListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for QuotaUsedArtifactListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for QuotaUsedArtifactListHeaders {}
impl crate::CountHeader for QuotaUsedArtifactListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct QuotaUsedAttachmentListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for QuotaUsedAttachmentListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for QuotaUsedAttachmentListHeaders {}
impl crate::CountHeader for QuotaUsedAttachmentListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct QuotaUsedPackageListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for QuotaUsedPackageListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for QuotaUsedPackageListHeaders {}
impl crate::CountHeader for QuotaUsedPackageListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct ReactionListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for ReactionListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for ReactionListHeaders {}
impl crate::CountHeader for ReactionListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct ReleaseListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for ReleaseListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for ReleaseListHeaders {}
impl crate::CountHeader for ReleaseListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct RepositoryListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for RepositoryListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for RepositoryListHeaders {}
impl crate::CountHeader for RepositoryListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct SecretListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for SecretListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for SecretListHeaders {}
impl crate::CountHeader for SecretListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct StopWatchListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for StopWatchListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for StopWatchListHeaders {}
impl crate::CountHeader for StopWatchListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct TagListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for TagListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for TagListHeaders {}
impl crate::CountHeader for TagListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct TeamListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for TeamListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for TeamListHeaders {}
impl crate::CountHeader for TeamListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct TimelineListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for TimelineListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for TimelineListHeaders {}
impl crate::CountHeader for TimelineListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct TrackedTimeListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for TrackedTimeListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for TrackedTimeListHeaders {}
impl crate::CountHeader for TrackedTimeListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct UserListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for UserListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for UserListHeaders {}
impl crate::CountHeader for UserListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct VariableListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for VariableListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for VariableListHeaders {}
impl crate::CountHeader for VariableListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct WikiCommitListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for WikiCommitListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for WikiCommitListHeaders {}
impl crate::CountHeader for WikiCommitListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct WikiPageListHeaders {
    pub x_total_count: Option<i64>,
}

impl TryFrom<&reqwest::header::HeaderMap> for WikiPageListHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let x_total_count = map
            .get("x-total-count")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        Ok(Self { x_total_count })
    }
}

impl crate::sealed::Sealed for WikiPageListHeaders {}
impl crate::CountHeader for WikiPageListHeaders {
    fn count(&self) -> Option<usize> {
        self.x_total_count.map(|x| x as usize)
    }
}

pub struct QuotaExceededHeaders {
    pub message: Option<String>,
    pub user_id: Option<i64>,
    pub username: Option<String>,
}

impl TryFrom<&reqwest::header::HeaderMap> for QuotaExceededHeaders {
    type Error = StructureError;

    fn try_from(map: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        let message = map
            .get("message")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                Ok::<_, StructureError>(s.to_string())
            })
            .transpose()?;
        let user_id = map
            .get("user_id")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                s.parse::<i64>()
                    .map_err(|_| StructureError::HeaderParseFailed)
            })
            .transpose()?;
        let username = map
            .get("username")
            .map(|s| -> Result<_, _> {
                let s = s.to_str().map_err(|_| StructureError::HeaderNotAscii)?;
                Ok::<_, StructureError>(s.to_string())
            })
            .transpose()?;
        Ok(Self {
            message,
            user_id,
            username,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdminSearchEmailsQuery {
    /// keyword
    pub q: Option<String>,
}

impl AdminSearchEmailsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(q) = self.q {
            list.push(("q", q));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdminSearchRunJobsQuery {
    /// a comma separated list of run job labels to search for
    pub labels: Option<String>,
}

impl AdminSearchRunJobsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(labels) = self.labels {
            list.push(("labels", labels));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdminUnadoptedListQuery {
    /// pattern of repositories to search for
    pub pattern: Option<String>,
}

impl AdminUnadoptedListQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(pattern) = self.pattern {
            list.push(("pattern", pattern));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdminSearchUsersQuery {
    /// ID of the user's login source to search for
    pub source_id: Option<i64>,
    /// user's login name to search for
    pub login_name: Option<String>,
    /// sort order of results
    pub sort: Option<AdminSearchUsersQuerySort>,
}

impl AdminSearchUsersQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(source_id) = self.source_id {
            list.push(("source_id", source_id.to_string()));
        }
        if let Some(login_name) = self.login_name {
            list.push(("login_name", login_name));
        }
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AdminSearchUsersQuerySort {
    #[serde(rename = "oldest")]
    Oldest,
    #[serde(rename = "newest")]
    Newest,
    #[serde(rename = "alphabetically")]
    Alphabetically,
    #[serde(rename = "reversealphabetically")]
    Reversealphabetically,
    #[serde(rename = "recentupdate")]
    Recentupdate,
    #[serde(rename = "leastupdate")]
    Leastupdate,
}

impl AdminSearchUsersQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            AdminSearchUsersQuerySort::Oldest => "oldest",
            AdminSearchUsersQuerySort::Newest => "newest",
            AdminSearchUsersQuerySort::Alphabetically => "alphabetically",
            AdminSearchUsersQuerySort::Reversealphabetically => "reversealphabetically",
            AdminSearchUsersQuerySort::Recentupdate => "recentupdate",
            AdminSearchUsersQuerySort::Leastupdate => "leastupdate",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdminDeleteUserQuery {
    /// purge the user from the system completely
    pub purge: Option<bool>,
}

impl AdminDeleteUserQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(purge) = self.purge {
            list.push(("purge", purge.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NotifyGetListQuery {
    /// If true, show notifications marked as read. Default value is false
    pub all: Option<bool>,
    /// Show notifications with the provided status types. Options are: unread, read and/or pinned. Defaults to unread & pinned.
    pub status_types: Option<Vec<String>>,
    /// filter notifications by subject type
    pub subject_type: Option<Vec<NotifyGetListQuerySubjectType>>,
    /// Only show notifications updated after the given time. This is a timestamp in RFC 3339 format
    pub since: Option<time::OffsetDateTime>,
    /// Only show notifications updated before the given time. This is a timestamp in RFC 3339 format
    pub before: Option<time::OffsetDateTime>,
}

impl NotifyGetListQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(all) = self.all {
            list.push(("all", all.to_string()));
        }
        if let Some(status_types) = self.status_types {
            if !status_types.is_empty() {
                for item in status_types {
                    list.push(("status-types", item.to_string()));
                }
            }
        }
        if let Some(subject_type) = self.subject_type {
            if !subject_type.is_empty() {
                for item in subject_type {
                    list.push(("subject-type", item.as_str().to_string()));
                }
            }
        }
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NotifyGetListQuerySubjectType {
    #[serde(rename = "issue")]
    Issue,
    #[serde(rename = "pull")]
    Pull,
    #[serde(rename = "repository")]
    Repository,
}

impl NotifyGetListQuerySubjectType {
    fn as_str(&self) -> &'static str {
        match self {
            NotifyGetListQuerySubjectType::Issue => "issue",
            NotifyGetListQuerySubjectType::Pull => "pull",
            NotifyGetListQuerySubjectType::Repository => "repository",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NotifyReadListQuery {
    /// Describes the last point that notifications were checked. Anything updated since this time will not be updated.
    pub last_read_at: Option<time::OffsetDateTime>,
    /// If true, mark all notifications on this repo. Default value is false
    pub all: Option<bool>,
    /// Mark notifications with the provided status types. Options are: unread, read and/or pinned. Defaults to unread.
    pub status_types: Option<Vec<String>>,
    /// Status to mark notifications as, Defaults to read.
    pub to_status: Option<String>,
}

impl NotifyReadListQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(last_read_at) = self.last_read_at {
            list.push((
                "last_read_at",
                last_read_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(all) = self.all {
            list.push(("all", all.to_string()));
        }
        if let Some(status_types) = self.status_types {
            if !status_types.is_empty() {
                for item in status_types {
                    list.push(("status-types", item.to_string()));
                }
            }
        }
        if let Some(to_status) = self.to_status {
            list.push(("to-status", to_status));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NotifyReadThreadQuery {
    /// Status to mark notifications as
    pub to_status: Option<String>,
}

impl NotifyReadThreadQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(to_status) = self.to_status {
            list.push(("to-status", to_status));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrgSearchRunJobsQuery {
    /// a comma separated list of run job labels to search for
    pub labels: Option<String>,
}

impl OrgSearchRunJobsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(labels) = self.labels {
            list.push(("labels", labels));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrgListActivityFeedsQuery {
    /// the date of the activities to be found
    pub date: Option<time::Date>,
}

impl OrgListActivityFeedsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(date) = self.date {
            list.push(("date", date.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrgListLabelsQuery {
    /// Specifies the sorting method: mostissues, leastissues, or reversealphabetically.
    pub sort: Option<OrgListLabelsQuerySort>,
}

impl OrgListLabelsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OrgListLabelsQuerySort {
    #[serde(rename = "mostissues")]
    Mostissues,
    #[serde(rename = "leastissues")]
    Leastissues,
    #[serde(rename = "reversealphabetically")]
    Reversealphabetically,
}

impl OrgListLabelsQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            OrgListLabelsQuerySort::Mostissues => "mostissues",
            OrgListLabelsQuerySort::Leastissues => "leastissues",
            OrgListLabelsQuerySort::Reversealphabetically => "reversealphabetically",
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct OrgCheckQuotaQuery {
    /// subject of the quota
    pub subject: String,
}

impl OrgCheckQuotaQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        let subject = self.subject;
        list.push(("subject", subject));

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamSearchQuery {
    /// keywords to search
    pub q: Option<String>,
    /// include search within team description (defaults to true)
    pub include_desc: Option<bool>,
}

impl TeamSearchQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(q) = self.q {
            list.push(("q", q));
        }
        if let Some(include_desc) = self.include_desc {
            list.push(("include_desc", include_desc.to_string()));
        }

        list
    }
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TeamSearchResults {
    pub data: Option<Vec<Team>>,
    pub ok: Option<bool>,
}

impl_from_response!(TeamSearchResults);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListPackagesQuery {
    /// package type filter
    pub r#type: Option<ListPackagesQueryType>,
    /// name filter
    pub q: Option<String>,
}

impl ListPackagesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(r#type) = self.r#type {
            list.push(("type", r#type.as_str().to_string()));
        }
        if let Some(q) = self.q {
            list.push(("q", q));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ListPackagesQueryType {
    #[serde(rename = "alpine")]
    Alpine,
    #[serde(rename = "cargo")]
    Cargo,
    #[serde(rename = "chef")]
    Chef,
    #[serde(rename = "composer")]
    Composer,
    #[serde(rename = "conan")]
    Conan,
    #[serde(rename = "conda")]
    Conda,
    #[serde(rename = "container")]
    Container,
    #[serde(rename = "cran")]
    Cran,
    #[serde(rename = "debian")]
    Debian,
    #[serde(rename = "generic")]
    Generic,
    #[serde(rename = "go")]
    Go,
    #[serde(rename = "helm")]
    Helm,
    #[serde(rename = "maven")]
    Maven,
    #[serde(rename = "npm")]
    Npm,
    #[serde(rename = "nuget")]
    Nuget,
    #[serde(rename = "pub")]
    Pub,
    #[serde(rename = "pypi")]
    Pypi,
    #[serde(rename = "rpm")]
    Rpm,
    #[serde(rename = "rubygems")]
    Rubygems,
    #[serde(rename = "swift")]
    Swift,
    #[serde(rename = "vagrant")]
    Vagrant,
}

impl ListPackagesQueryType {
    fn as_str(&self) -> &'static str {
        match self {
            ListPackagesQueryType::Alpine => "alpine",
            ListPackagesQueryType::Cargo => "cargo",
            ListPackagesQueryType::Chef => "chef",
            ListPackagesQueryType::Composer => "composer",
            ListPackagesQueryType::Conan => "conan",
            ListPackagesQueryType::Conda => "conda",
            ListPackagesQueryType::Container => "container",
            ListPackagesQueryType::Cran => "cran",
            ListPackagesQueryType::Debian => "debian",
            ListPackagesQueryType::Generic => "generic",
            ListPackagesQueryType::Go => "go",
            ListPackagesQueryType::Helm => "helm",
            ListPackagesQueryType::Maven => "maven",
            ListPackagesQueryType::Npm => "npm",
            ListPackagesQueryType::Nuget => "nuget",
            ListPackagesQueryType::Pub => "pub",
            ListPackagesQueryType::Pypi => "pypi",
            ListPackagesQueryType::Rpm => "rpm",
            ListPackagesQueryType::Rubygems => "rubygems",
            ListPackagesQueryType::Swift => "swift",
            ListPackagesQueryType::Vagrant => "vagrant",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueSearchIssuesQuery {
    /// State of the issue
    pub state: Option<IssueSearchIssuesQueryState>,
    /// Comma-separated list of label names. Fetch only issues that have any of these labels. Non existent labels are discarded.
    pub labels: Option<String>,
    /// Comma-separated list of milestone names. Fetch only issues that have any of these milestones. Non existent milestones are discarded.
    pub milestones: Option<String>,
    /// Search string
    pub q: Option<String>,
    /// Repository ID to prioritize in the results
    pub priority_repo_id: Option<i64>,
    /// Filter by issue type
    pub r#type: Option<IssueSearchIssuesQueryType>,
    /// Only show issues updated after the given time (RFC 3339 format)
    pub since: Option<time::OffsetDateTime>,
    /// Only show issues updated before the given time (RFC 3339 format)
    pub before: Option<time::OffsetDateTime>,
    /// Filter issues or pulls assigned to the authenticated user
    pub assigned: Option<bool>,
    /// Filter issues or pulls created by the authenticated user
    pub created: Option<bool>,
    /// Filter issues or pulls mentioning the authenticated user
    pub mentioned: Option<bool>,
    /// Filter pull requests where the authenticated user's review was requested
    pub review_requested: Option<bool>,
    /// Filter pull requests reviewed by the authenticated user
    pub reviewed: Option<bool>,
    /// Filter by repository owner
    pub owner: Option<String>,
    /// Filter by team (requires organization owner parameter)
    pub team: Option<String>,
    /// Type of sort
    pub sort: Option<IssueSearchIssuesQuerySort>,
}

impl IssueSearchIssuesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(state) = self.state {
            list.push(("state", state.as_str().to_string()));
        }
        if let Some(labels) = self.labels {
            list.push(("labels", labels));
        }
        if let Some(milestones) = self.milestones {
            list.push(("milestones", milestones));
        }
        if let Some(q) = self.q {
            list.push(("q", q));
        }
        if let Some(priority_repo_id) = self.priority_repo_id {
            list.push(("priority_repo_id", priority_repo_id.to_string()));
        }
        if let Some(r#type) = self.r#type {
            list.push(("type", r#type.as_str().to_string()));
        }
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(assigned) = self.assigned {
            list.push(("assigned", assigned.to_string()));
        }
        if let Some(created) = self.created {
            list.push(("created", created.to_string()));
        }
        if let Some(mentioned) = self.mentioned {
            list.push(("mentioned", mentioned.to_string()));
        }
        if let Some(review_requested) = self.review_requested {
            list.push(("review_requested", review_requested.to_string()));
        }
        if let Some(reviewed) = self.reviewed {
            list.push(("reviewed", reviewed.to_string()));
        }
        if let Some(owner) = self.owner {
            list.push(("owner", owner));
        }
        if let Some(team) = self.team {
            list.push(("team", team));
        }
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueSearchIssuesQueryState {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "all")]
    All,
}

impl IssueSearchIssuesQueryState {
    fn as_str(&self) -> &'static str {
        match self {
            IssueSearchIssuesQueryState::Open => "open",
            IssueSearchIssuesQueryState::Closed => "closed",
            IssueSearchIssuesQueryState::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueSearchIssuesQueryType {
    #[serde(rename = "issues")]
    Issues,
    #[serde(rename = "pulls")]
    Pulls,
}

impl IssueSearchIssuesQueryType {
    fn as_str(&self) -> &'static str {
        match self {
            IssueSearchIssuesQueryType::Issues => "issues",
            IssueSearchIssuesQueryType::Pulls => "pulls",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueSearchIssuesQuerySort {
    #[serde(rename = "relevance")]
    Relevance,
    #[serde(rename = "latest")]
    Latest,
    #[serde(rename = "oldest")]
    Oldest,
    #[serde(rename = "recentupdate")]
    Recentupdate,
    #[serde(rename = "leastupdate")]
    Leastupdate,
    #[serde(rename = "mostcomment")]
    Mostcomment,
    #[serde(rename = "leastcomment")]
    Leastcomment,
    #[serde(rename = "nearduedate")]
    Nearduedate,
    #[serde(rename = "farduedate")]
    Farduedate,
}

impl IssueSearchIssuesQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            IssueSearchIssuesQuerySort::Relevance => "relevance",
            IssueSearchIssuesQuerySort::Latest => "latest",
            IssueSearchIssuesQuerySort::Oldest => "oldest",
            IssueSearchIssuesQuerySort::Recentupdate => "recentupdate",
            IssueSearchIssuesQuerySort::Leastupdate => "leastupdate",
            IssueSearchIssuesQuerySort::Mostcomment => "mostcomment",
            IssueSearchIssuesQuerySort::Leastcomment => "leastcomment",
            IssueSearchIssuesQuerySort::Nearduedate => "nearduedate",
            IssueSearchIssuesQuerySort::Farduedate => "farduedate",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoSearchQuery {
    /// keyword
    pub q: Option<String>,
    /// Limit search to repositories with keyword as topic
    pub topic: Option<bool>,
    /// include search of keyword within repository description
    pub include_desc: Option<bool>,
    /// search only for repos that the user with the given id owns or contributes to
    pub uid: Option<i64>,
    /// repo owner to prioritize in the results
    pub priority_owner_id: Option<i64>,
    /// search only for repos that belong to the given team id
    pub team_id: Option<i64>,
    /// search only for repos that the user with the given id has starred
    pub starred_by: Option<i64>,
    /// include private repositories this user has access to (defaults to true)
    pub private: Option<bool>,
    /// show only public, private or all repositories (defaults to all)
    pub is_private: Option<bool>,
    /// include template repositories this user has access to (defaults to true)
    pub template: Option<bool>,
    /// show only archived, non-archived or all repositories (defaults to all)
    pub archived: Option<bool>,
    /// type of repository to search for. Supported values are "fork", "source", "mirror" and "collaborative"
    pub mode: Option<String>,
    /// if `uid` is given, search only for repos that the user owns
    pub exclusive: Option<bool>,
    /// sort repos by attribute. Supported values are "alpha", "created", "updated", "size", "git_size", "lfs_size", "stars", "forks" and "id". Default is "alpha"
    pub sort: Option<RepoSearchQuerySort>,
    /// sort order, either "asc" (ascending) or "desc" (descending). Default is "asc", ignored if "sort" is not specified.
    pub order: Option<RepoSearchQueryOrder>,
}

impl RepoSearchQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(q) = self.q {
            list.push(("q", q));
        }
        if let Some(topic) = self.topic {
            list.push(("topic", topic.to_string()));
        }
        if let Some(include_desc) = self.include_desc {
            list.push(("includeDesc", include_desc.to_string()));
        }
        if let Some(uid) = self.uid {
            list.push(("uid", uid.to_string()));
        }
        if let Some(priority_owner_id) = self.priority_owner_id {
            list.push(("priority_owner_id", priority_owner_id.to_string()));
        }
        if let Some(team_id) = self.team_id {
            list.push(("team_id", team_id.to_string()));
        }
        if let Some(starred_by) = self.starred_by {
            list.push(("starredBy", starred_by.to_string()));
        }
        if let Some(private) = self.private {
            list.push(("private", private.to_string()));
        }
        if let Some(is_private) = self.is_private {
            list.push(("is_private", is_private.to_string()));
        }
        if let Some(template) = self.template {
            list.push(("template", template.to_string()));
        }
        if let Some(archived) = self.archived {
            list.push(("archived", archived.to_string()));
        }
        if let Some(mode) = self.mode {
            list.push(("mode", mode));
        }
        if let Some(exclusive) = self.exclusive {
            list.push(("exclusive", exclusive.to_string()));
        }
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }
        if let Some(order) = self.order {
            list.push(("order", order.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoSearchQuerySort {
    #[serde(rename = "alpha")]
    Alpha,
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "updated")]
    Updated,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "git_size")]
    GitSize,
    #[serde(rename = "lfs_size")]
    LfsSize,
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "stars")]
    Stars,
    #[serde(rename = "forks")]
    Forks,
}

impl RepoSearchQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            RepoSearchQuerySort::Alpha => "alpha",
            RepoSearchQuerySort::Created => "created",
            RepoSearchQuerySort::Updated => "updated",
            RepoSearchQuerySort::Size => "size",
            RepoSearchQuerySort::GitSize => "git_size",
            RepoSearchQuerySort::LfsSize => "lfs_size",
            RepoSearchQuerySort::Id => "id",
            RepoSearchQuerySort::Stars => "stars",
            RepoSearchQuerySort::Forks => "forks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoSearchQueryOrder {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

impl RepoSearchQueryOrder {
    fn as_str(&self) -> &'static str {
        match self {
            RepoSearchQueryOrder::Asc => "asc",
            RepoSearchQueryOrder::Desc => "desc",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoSearchRunJobsQuery {
    /// a comma separated list of run job labels to search for
    pub labels: Option<String>,
}

impl RepoSearchRunJobsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(labels) = self.labels {
            list.push(("labels", labels));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListActionRunsQuery {
    /// Returns workflow run triggered by the specified events. For example, `push`, `pull_request` or `workflow_dispatch`.
    pub event: Option<Vec<String>>,
    /// Returns workflow runs with the check run status or conclusion that is specified. For example, a conclusion can be success or a status can be in_progress. Only Forgejo Actions can set a status of waiting, pending, or requested.
    pub status: Option<Vec<ListActionRunsQueryStatus>>,
    /// Returns the workflow run associated with the run number.
    pub run_number: Option<i64>,
    /// Only returns workflow runs that are associated with the specified head_sha.
    pub head_sha: Option<String>,
}

impl ListActionRunsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(event) = self.event {
            if !event.is_empty() {
                let mut s = String::new();
                for (i, item) in event.iter().enumerate() {
                    s.push_str(item);
                    if i < event.len() - 1 {
                        s.push(',')
                    }
                }
                list.push(("event", s));
            }
        }
        if let Some(status) = self.status {
            if !status.is_empty() {
                let mut s = String::new();
                for (i, item) in status.iter().enumerate() {
                    s.push_str(item.as_str());
                    if i < status.len() - 1 {
                        s.push(',')
                    }
                }
                list.push(("status", s));
            }
        }
        if let Some(run_number) = self.run_number {
            list.push(("run_number", run_number.to_string()));
        }
        if let Some(head_sha) = self.head_sha {
            list.push(("head_sha", head_sha));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ListActionRunsQueryStatus {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "skipped")]
    Skipped,
    #[serde(rename = "blocked")]
    Blocked,
}

impl ListActionRunsQueryStatus {
    fn as_str(&self) -> &'static str {
        match self {
            ListActionRunsQueryStatus::Unknown => "unknown",
            ListActionRunsQueryStatus::Waiting => "waiting",
            ListActionRunsQueryStatus::Running => "running",
            ListActionRunsQueryStatus::Success => "success",
            ListActionRunsQueryStatus::Failure => "failure",
            ListActionRunsQueryStatus::Cancelled => "cancelled",
            ListActionRunsQueryStatus::Skipped => "skipped",
            ListActionRunsQueryStatus::Blocked => "blocked",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoListActivityFeedsQuery {
    /// the date of the activities to be found
    pub date: Option<time::Date>,
}

impl RepoListActivityFeedsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(date) = self.date {
            list.push(("date", date.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetAllCommitsQuery {
    /// SHA or branch to start listing commits from (usually 'master')
    pub sha: Option<String>,
    /// filepath of a file/dir
    pub path: Option<String>,
    /// include diff stats for every commit (disable for speedup, default 'true')
    pub stat: Option<bool>,
    /// include verification for every commit (disable for speedup, default 'true')
    pub verification: Option<bool>,
    /// include a list of affected files for every commit (disable for speedup, default 'true')
    pub files: Option<bool>,
    /// commits that match the given specifier will not be listed.
    pub not: Option<String>,
}

impl RepoGetAllCommitsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(sha) = self.sha {
            list.push(("sha", sha));
        }
        if let Some(path) = self.path {
            list.push(("path", path));
        }
        if let Some(stat) = self.stat {
            list.push(("stat", stat.to_string()));
        }
        if let Some(verification) = self.verification {
            list.push(("verification", verification.to_string()));
        }
        if let Some(files) = self.files {
            list.push(("files", files.to_string()));
        }
        if let Some(not) = self.not {
            list.push(("not", not));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoListStatusesByRefQuery {
    /// type of sort
    pub sort: Option<RepoListStatusesByRefQuerySort>,
    /// type of state
    pub state: Option<RepoListStatusesByRefQueryState>,
}

impl RepoListStatusesByRefQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }
        if let Some(state) = self.state {
            list.push(("state", state.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoListStatusesByRefQuerySort {
    #[serde(rename = "oldest")]
    Oldest,
    #[serde(rename = "recentupdate")]
    Recentupdate,
    #[serde(rename = "leastupdate")]
    Leastupdate,
    #[serde(rename = "leastindex")]
    Leastindex,
    #[serde(rename = "highestindex")]
    Highestindex,
}

impl RepoListStatusesByRefQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            RepoListStatusesByRefQuerySort::Oldest => "oldest",
            RepoListStatusesByRefQuerySort::Recentupdate => "recentupdate",
            RepoListStatusesByRefQuerySort::Leastupdate => "leastupdate",
            RepoListStatusesByRefQuerySort::Leastindex => "leastindex",
            RepoListStatusesByRefQuerySort::Highestindex => "highestindex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoListStatusesByRefQueryState {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "failure")]
    Failure,
    #[serde(rename = "warning")]
    Warning,
}

impl RepoListStatusesByRefQueryState {
    fn as_str(&self) -> &'static str {
        match self {
            RepoListStatusesByRefQueryState::Pending => "pending",
            RepoListStatusesByRefQueryState::Success => "success",
            RepoListStatusesByRefQueryState::Error => "error",
            RepoListStatusesByRefQueryState::Failure => "failure",
            RepoListStatusesByRefQueryState::Warning => "warning",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetContentsListQuery {
    /// The name of the commit/branch/tag. Default the repository’s default branch (usually master)
    pub r#ref: Option<String>,
}

impl RepoGetContentsListQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(r#ref) = self.r#ref {
            list.push(("ref", r#ref));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetContentsQuery {
    /// The name of the commit/branch/tag. Default the repository’s default branch (usually master)
    pub r#ref: Option<String>,
}

impl RepoGetContentsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(r#ref) = self.r#ref {
            list.push(("ref", r#ref));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetEditorConfigQuery {
    /// The name of the commit/branch/tag. Default the repository’s default branch (usually master)
    pub r#ref: Option<String>,
}

impl RepoGetEditorConfigQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(r#ref) = self.r#ref {
            list.push(("ref", r#ref));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GetBlobsQuery {
    /// a comma separated list of blob-sha (mind the overall URL-length limit of ~2,083 chars)
    pub shas: String,
}

impl GetBlobsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        let shas = self.shas;
        list.push(("shas", shas));

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetSingleCommitQuery {
    /// include diff stats for every commit (disable for speedup, default 'true')
    pub stat: Option<bool>,
    /// include verification for every commit (disable for speedup, default 'true')
    pub verification: Option<bool>,
    /// include a list of affected files for every commit (disable for speedup, default 'true')
    pub files: Option<bool>,
}

impl RepoGetSingleCommitQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(stat) = self.stat {
            list.push(("stat", stat.to_string()));
        }
        if let Some(verification) = self.verification {
            list.push(("verification", verification.to_string()));
        }
        if let Some(files) = self.files {
            list.push(("files", files.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetNoteQuery {
    /// include verification for every commit (disable for speedup, default 'true')
    pub verification: Option<bool>,
    /// include a list of affected files for every commit (disable for speedup, default 'true')
    pub files: Option<bool>,
}

impl RepoGetNoteQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(verification) = self.verification {
            list.push(("verification", verification.to_string()));
        }
        if let Some(files) = self.files {
            list.push(("files", files.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GetTreeQuery {
    /// show all directories and files
    pub recursive: Option<bool>,
    /// number of items per page
    pub per_page: Option<u32>,
}

impl GetTreeQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(recursive) = self.recursive {
            list.push(("recursive", recursive.to_string()));
        }
        if let Some(per_page) = self.per_page {
            list.push(("per_page", per_page.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoTestHookQuery {
    /// The name of the commit/branch/tag, indicates which commit will be loaded to the webhook payload.
    pub r#ref: Option<String>,
}

impl RepoTestHookQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(r#ref) = self.r#ref {
            list.push(("ref", r#ref));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueListIssuesQuery {
    /// whether issue is open or closed
    pub state: Option<IssueListIssuesQueryState>,
    /// comma separated list of labels. Fetch only issues that have any of this labels. Non existent labels are discarded
    pub labels: Option<String>,
    /// search string
    pub q: Option<String>,
    /// filter by type (issues / pulls) if set
    pub r#type: Option<IssueListIssuesQueryType>,
    /// comma separated list of milestone names or ids. It uses names and fall back to ids. Fetch only issues that have any of this milestones. Non existent milestones are discarded
    pub milestones: Option<String>,
    /// Only show items updated after the given time. This is a timestamp in RFC 3339 format
    pub since: Option<time::OffsetDateTime>,
    /// Only show items updated before the given time. This is a timestamp in RFC 3339 format
    pub before: Option<time::OffsetDateTime>,
    /// Only show items which were created by the given user
    pub created_by: Option<String>,
    /// Only show items for which the given user is assigned
    pub assigned_by: Option<String>,
    /// Only show items in which the given user was mentioned
    pub mentioned_by: Option<String>,
    /// Type of sort
    pub sort: Option<IssueListIssuesQuerySort>,
}

impl IssueListIssuesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(state) = self.state {
            list.push(("state", state.as_str().to_string()));
        }
        if let Some(labels) = self.labels {
            list.push(("labels", labels));
        }
        if let Some(q) = self.q {
            list.push(("q", q));
        }
        if let Some(r#type) = self.r#type {
            list.push(("type", r#type.as_str().to_string()));
        }
        if let Some(milestones) = self.milestones {
            list.push(("milestones", milestones));
        }
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(created_by) = self.created_by {
            list.push(("created_by", created_by));
        }
        if let Some(assigned_by) = self.assigned_by {
            list.push(("assigned_by", assigned_by));
        }
        if let Some(mentioned_by) = self.mentioned_by {
            list.push(("mentioned_by", mentioned_by));
        }
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueListIssuesQueryState {
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "all")]
    All,
}

impl IssueListIssuesQueryState {
    fn as_str(&self) -> &'static str {
        match self {
            IssueListIssuesQueryState::Closed => "closed",
            IssueListIssuesQueryState::Open => "open",
            IssueListIssuesQueryState::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueListIssuesQueryType {
    #[serde(rename = "issues")]
    Issues,
    #[serde(rename = "pulls")]
    Pulls,
}

impl IssueListIssuesQueryType {
    fn as_str(&self) -> &'static str {
        match self {
            IssueListIssuesQueryType::Issues => "issues",
            IssueListIssuesQueryType::Pulls => "pulls",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueListIssuesQuerySort {
    #[serde(rename = "relevance")]
    Relevance,
    #[serde(rename = "latest")]
    Latest,
    #[serde(rename = "oldest")]
    Oldest,
    #[serde(rename = "recentupdate")]
    Recentupdate,
    #[serde(rename = "leastupdate")]
    Leastupdate,
    #[serde(rename = "mostcomment")]
    Mostcomment,
    #[serde(rename = "leastcomment")]
    Leastcomment,
    #[serde(rename = "nearduedate")]
    Nearduedate,
    #[serde(rename = "farduedate")]
    Farduedate,
}

impl IssueListIssuesQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            IssueListIssuesQuerySort::Relevance => "relevance",
            IssueListIssuesQuerySort::Latest => "latest",
            IssueListIssuesQuerySort::Oldest => "oldest",
            IssueListIssuesQuerySort::Recentupdate => "recentupdate",
            IssueListIssuesQuerySort::Leastupdate => "leastupdate",
            IssueListIssuesQuerySort::Mostcomment => "mostcomment",
            IssueListIssuesQuerySort::Leastcomment => "leastcomment",
            IssueListIssuesQuerySort::Nearduedate => "nearduedate",
            IssueListIssuesQuerySort::Farduedate => "farduedate",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueGetRepoCommentsQuery {
    /// if provided, only comments updated since the provided time are returned.
    pub since: Option<time::OffsetDateTime>,
    /// if provided, only comments updated before the provided time are returned.
    pub before: Option<time::OffsetDateTime>,
}

impl IssueGetRepoCommentsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueCreateIssueCommentAttachmentQuery {
    /// name of the attachment
    pub name: Option<String>,
    /// time of the attachment's creation. This is a timestamp in RFC 3339 format
    pub updated_at: Option<time::OffsetDateTime>,
}

impl IssueCreateIssueCommentAttachmentQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(name) = self.name {
            list.push(("name", name));
        }
        if let Some(updated_at) = self.updated_at {
            list.push((
                "updated_at",
                updated_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueCreateIssueAttachmentQuery {
    /// name of the attachment
    pub name: Option<String>,
    /// time of the attachment's creation. This is a timestamp in RFC 3339 format
    pub updated_at: Option<time::OffsetDateTime>,
}

impl IssueCreateIssueAttachmentQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(name) = self.name {
            list.push(("name", name));
        }
        if let Some(updated_at) = self.updated_at {
            list.push((
                "updated_at",
                updated_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueGetCommentsQuery {
    /// if provided, only comments updated since the specified time are returned.
    pub since: Option<time::OffsetDateTime>,
    /// if provided, only comments updated before the provided time are returned.
    pub before: Option<time::OffsetDateTime>,
}

impl IssueGetCommentsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueGetCommentsAndTimelineQuery {
    /// if provided, only comments updated since the specified time are returned.
    pub since: Option<time::OffsetDateTime>,
    /// if provided, only comments updated before the provided time are returned.
    pub before: Option<time::OffsetDateTime>,
}

impl IssueGetCommentsAndTimelineQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueTrackedTimesQuery {
    /// optional filter by user (available for issue managers)
    pub user: Option<String>,
    /// Only show times updated after the given time. This is a timestamp in RFC 3339 format
    pub since: Option<time::OffsetDateTime>,
    /// Only show times updated before the given time. This is a timestamp in RFC 3339 format
    pub before: Option<time::OffsetDateTime>,
}

impl IssueTrackedTimesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(user) = self.user {
            list.push(("user", user));
        }
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoListKeysQuery {
    /// the key_id to search for
    pub key_id: Option<u32>,
    /// fingerprint of the key
    pub fingerprint: Option<String>,
}

impl RepoListKeysQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(key_id) = self.key_id {
            list.push(("key_id", key_id.to_string()));
        }
        if let Some(fingerprint) = self.fingerprint {
            list.push(("fingerprint", fingerprint));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueListLabelsQuery {
    /// Specifies the sorting method: mostissues, leastissues, or reversealphabetically.
    pub sort: Option<IssueListLabelsQuerySort>,
}

impl IssueListLabelsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueListLabelsQuerySort {
    #[serde(rename = "mostissues")]
    Mostissues,
    #[serde(rename = "leastissues")]
    Leastissues,
    #[serde(rename = "reversealphabetically")]
    Reversealphabetically,
}

impl IssueListLabelsQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            IssueListLabelsQuerySort::Mostissues => "mostissues",
            IssueListLabelsQuerySort::Leastissues => "leastissues",
            IssueListLabelsQuerySort::Reversealphabetically => "reversealphabetically",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetRawFileOrLfsQuery {
    /// The name of the commit/branch/tag. Default the repository’s default branch (usually master)
    pub r#ref: Option<String>,
}

impl RepoGetRawFileOrLfsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(r#ref) = self.r#ref {
            list.push(("ref", r#ref));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IssueGetMilestonesListQuery {
    /// Milestone state, Recognized values are open, closed and all. Defaults to "open"
    pub state: Option<String>,
    /// filter by milestone name
    pub name: Option<String>,
}

impl IssueGetMilestonesListQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(state) = self.state {
            list.push(("state", state));
        }
        if let Some(name) = self.name {
            list.push(("name", name));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NotifyGetRepoListQuery {
    /// If true, show notifications marked as read. Default value is false
    pub all: Option<bool>,
    /// Show notifications with the provided status types. Options are: unread, read and/or pinned. Defaults to unread & pinned
    pub status_types: Option<Vec<String>>,
    /// filter notifications by subject type
    pub subject_type: Option<Vec<NotifyGetRepoListQuerySubjectType>>,
    /// Only show notifications updated after the given time. This is a timestamp in RFC 3339 format
    pub since: Option<time::OffsetDateTime>,
    /// Only show notifications updated before the given time. This is a timestamp in RFC 3339 format
    pub before: Option<time::OffsetDateTime>,
}

impl NotifyGetRepoListQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(all) = self.all {
            list.push(("all", all.to_string()));
        }
        if let Some(status_types) = self.status_types {
            if !status_types.is_empty() {
                for item in status_types {
                    list.push(("status-types", item.to_string()));
                }
            }
        }
        if let Some(subject_type) = self.subject_type {
            if !subject_type.is_empty() {
                for item in subject_type {
                    list.push(("subject-type", item.as_str().to_string()));
                }
            }
        }
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NotifyGetRepoListQuerySubjectType {
    #[serde(rename = "issue")]
    Issue,
    #[serde(rename = "pull")]
    Pull,
    #[serde(rename = "repository")]
    Repository,
}

impl NotifyGetRepoListQuerySubjectType {
    fn as_str(&self) -> &'static str {
        match self {
            NotifyGetRepoListQuerySubjectType::Issue => "issue",
            NotifyGetRepoListQuerySubjectType::Pull => "pull",
            NotifyGetRepoListQuerySubjectType::Repository => "repository",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NotifyReadRepoListQuery {
    /// If true, mark all notifications on this repo. Default value is false
    pub all: Option<bool>,
    /// Mark notifications with the provided status types. Options are: unread, read and/or pinned. Defaults to unread.
    pub status_types: Option<Vec<String>>,
    /// Status to mark notifications as. Defaults to read.
    pub to_status: Option<String>,
    /// Describes the last point that notifications were checked. Anything updated since this time will not be updated.
    pub last_read_at: Option<time::OffsetDateTime>,
}

impl NotifyReadRepoListQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(all) = self.all {
            list.push(("all", all.to_string()));
        }
        if let Some(status_types) = self.status_types {
            if !status_types.is_empty() {
                for item in status_types {
                    list.push(("status-types", item.to_string()));
                }
            }
        }
        if let Some(to_status) = self.to_status {
            list.push(("to-status", to_status));
        }
        if let Some(last_read_at) = self.last_read_at {
            list.push((
                "last_read_at",
                last_read_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoListPullRequestsQuery {
    /// State of pull request
    pub state: Option<RepoListPullRequestsQueryState>,
    /// Type of sort
    pub sort: Option<RepoListPullRequestsQuerySort>,
    /// ID of the milestone
    pub milestone: Option<i64>,
    /// Label IDs
    pub labels: Option<Vec<i64>>,
    /// Filter by pull request author
    pub poster: Option<String>,
}

impl RepoListPullRequestsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(state) = self.state {
            list.push(("state", state.as_str().to_string()));
        }
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }
        if let Some(milestone) = self.milestone {
            list.push(("milestone", milestone.to_string()));
        }
        if let Some(labels) = self.labels {
            if !labels.is_empty() {
                for item in labels {
                    list.push(("labels", format!("{item}")));
                }
            }
        }
        if let Some(poster) = self.poster {
            list.push(("poster", poster));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoListPullRequestsQueryState {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "all")]
    All,
}

impl RepoListPullRequestsQueryState {
    fn as_str(&self) -> &'static str {
        match self {
            RepoListPullRequestsQueryState::Open => "open",
            RepoListPullRequestsQueryState::Closed => "closed",
            RepoListPullRequestsQueryState::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoListPullRequestsQuerySort {
    #[serde(rename = "oldest")]
    Oldest,
    #[serde(rename = "recentupdate")]
    Recentupdate,
    #[serde(rename = "recentclose")]
    Recentclose,
    #[serde(rename = "leastupdate")]
    Leastupdate,
    #[serde(rename = "mostcomment")]
    Mostcomment,
    #[serde(rename = "leastcomment")]
    Leastcomment,
    #[serde(rename = "priority")]
    Priority,
}

impl RepoListPullRequestsQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            RepoListPullRequestsQuerySort::Oldest => "oldest",
            RepoListPullRequestsQuerySort::Recentupdate => "recentupdate",
            RepoListPullRequestsQuerySort::Recentclose => "recentclose",
            RepoListPullRequestsQuerySort::Leastupdate => "leastupdate",
            RepoListPullRequestsQuerySort::Mostcomment => "mostcomment",
            RepoListPullRequestsQuerySort::Leastcomment => "leastcomment",
            RepoListPullRequestsQuerySort::Priority => "priority",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoDownloadPullDiffOrPatchQuery {
    /// whether to include binary file changes. if true, the diff is applicable with `git apply`
    pub binary: Option<bool>,
}

impl RepoDownloadPullDiffOrPatchQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(binary) = self.binary {
            list.push(("binary", binary.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetPullRequestCommitsQuery {
    /// include verification for every commit (disable for speedup, default 'true')
    pub verification: Option<bool>,
    /// include a list of affected files for every commit (disable for speedup, default 'true')
    pub files: Option<bool>,
}

impl RepoGetPullRequestCommitsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(verification) = self.verification {
            list.push(("verification", verification.to_string()));
        }
        if let Some(files) = self.files {
            list.push(("files", files.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetPullRequestFilesQuery {
    /// skip to given file
    pub skip_to: Option<String>,
    /// whitespace behavior
    pub whitespace: Option<RepoGetPullRequestFilesQueryWhitespace>,
}

impl RepoGetPullRequestFilesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(skip_to) = self.skip_to {
            list.push(("skip-to", skip_to));
        }
        if let Some(whitespace) = self.whitespace {
            list.push(("whitespace", whitespace.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoGetPullRequestFilesQueryWhitespace {
    #[serde(rename = "ignore-all")]
    IgnoreAll,
    #[serde(rename = "ignore-change")]
    IgnoreChange,
    #[serde(rename = "ignore-eol")]
    IgnoreEol,
    #[serde(rename = "show-all")]
    ShowAll,
}

impl RepoGetPullRequestFilesQueryWhitespace {
    fn as_str(&self) -> &'static str {
        match self {
            RepoGetPullRequestFilesQueryWhitespace::IgnoreAll => "ignore-all",
            RepoGetPullRequestFilesQueryWhitespace::IgnoreChange => "ignore-change",
            RepoGetPullRequestFilesQueryWhitespace::IgnoreEol => "ignore-eol",
            RepoGetPullRequestFilesQueryWhitespace::ShowAll => "show-all",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoUpdatePullRequestQuery {
    /// how to update pull request
    pub style: Option<RepoUpdatePullRequestQueryStyle>,
}

impl RepoUpdatePullRequestQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(style) = self.style {
            list.push(("style", style.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoUpdatePullRequestQueryStyle {
    #[serde(rename = "merge")]
    Merge,
    #[serde(rename = "rebase")]
    Rebase,
}

impl RepoUpdatePullRequestQueryStyle {
    fn as_str(&self) -> &'static str {
        match self {
            RepoUpdatePullRequestQueryStyle::Merge => "merge",
            RepoUpdatePullRequestQueryStyle::Rebase => "rebase",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoGetRawFileQuery {
    /// The name of the commit/branch/tag. Default the repository’s default branch (usually master)
    pub r#ref: Option<String>,
}

impl RepoGetRawFileQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(r#ref) = self.r#ref {
            list.push(("ref", r#ref));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoListReleasesQuery {
    /// filter (exclude / include) drafts, if you dont have repo write access none will show
    pub draft: Option<bool>,
    /// filter (exclude / include) pre-releases
    pub pre_release: Option<bool>,
    /// Search string
    pub q: Option<String>,
}

impl RepoListReleasesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(draft) = self.draft {
            list.push(("draft", draft.to_string()));
        }
        if let Some(pre_release) = self.pre_release {
            list.push(("pre-release", pre_release.to_string()));
        }
        if let Some(q) = self.q {
            list.push(("q", q));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoCreateReleaseAttachmentQuery {
    /// name of the attachment
    pub name: Option<String>,
}

impl RepoCreateReleaseAttachmentQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(name) = self.name {
            list.push(("name", name));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoListStatusesQuery {
    /// type of sort
    pub sort: Option<RepoListStatusesQuerySort>,
    /// type of state
    pub state: Option<RepoListStatusesQueryState>,
}

impl RepoListStatusesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }
        if let Some(state) = self.state {
            list.push(("state", state.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoListStatusesQuerySort {
    #[serde(rename = "oldest")]
    Oldest,
    #[serde(rename = "recentupdate")]
    Recentupdate,
    #[serde(rename = "leastupdate")]
    Leastupdate,
    #[serde(rename = "leastindex")]
    Leastindex,
    #[serde(rename = "highestindex")]
    Highestindex,
}

impl RepoListStatusesQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            RepoListStatusesQuerySort::Oldest => "oldest",
            RepoListStatusesQuerySort::Recentupdate => "recentupdate",
            RepoListStatusesQuerySort::Leastupdate => "leastupdate",
            RepoListStatusesQuerySort::Leastindex => "leastindex",
            RepoListStatusesQuerySort::Highestindex => "highestindex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepoListStatusesQueryState {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "failure")]
    Failure,
    #[serde(rename = "warning")]
    Warning,
}

impl RepoListStatusesQueryState {
    fn as_str(&self) -> &'static str {
        match self {
            RepoListStatusesQueryState::Pending => "pending",
            RepoListStatusesQueryState::Success => "success",
            RepoListStatusesQueryState::Error => "error",
            RepoListStatusesQueryState::Failure => "failure",
            RepoListStatusesQueryState::Warning => "warning",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoTrackedTimesQuery {
    /// optional filter by user (available for issue managers)
    pub user: Option<String>,
    /// Only show times updated after the given time. This is a timestamp in RFC 3339 format
    pub since: Option<time::OffsetDateTime>,
    /// Only show times updated before the given time. This is a timestamp in RFC 3339 format
    pub before: Option<time::OffsetDateTime>,
}

impl RepoTrackedTimesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(user) = self.user {
            list.push(("user", user));
        }
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrgListTeamActivityFeedsQuery {
    /// the date of the activities to be found
    pub date: Option<time::Date>,
}

impl OrgListTeamActivityFeedsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(date) = self.date {
            list.push(("date", date.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopicSearchQuery {
    /// keyword to search for
    pub q: String,
}

impl TopicSearchQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        let q = self.q;
        list.push(("q", q));

        list
    }
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TopicSearchResults {
    pub topics: Option<Vec<TopicResponse>>,
}

impl_from_response!(TopicSearchResults);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserSearchRunJobsQuery {
    /// a comma separated list of run job labels to search for
    pub labels: Option<String>,
}

impl UserSearchRunJobsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(labels) = self.labels {
            list.push(("labels", labels));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserCurrentListKeysQuery {
    /// fingerprint of the key
    pub fingerprint: Option<String>,
}

impl UserCurrentListKeysQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(fingerprint) = self.fingerprint {
            list.push(("fingerprint", fingerprint));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserCheckQuotaQuery {
    /// subject of the quota
    pub subject: String,
}

impl UserCheckQuotaQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        let subject = self.subject;
        list.push(("subject", subject));

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserCurrentListReposQuery {
    /// order the repositories
    pub order_by: Option<UserCurrentListReposQueryOrderBy>,
}

impl UserCurrentListReposQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(order_by) = self.order_by {
            list.push(("order_by", order_by.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UserCurrentListReposQueryOrderBy {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "newest")]
    Newest,
    #[serde(rename = "oldest")]
    Oldest,
    #[serde(rename = "recentupdate")]
    Recentupdate,
    #[serde(rename = "leastupdate")]
    Leastupdate,
    #[serde(rename = "reversealphabetically")]
    Reversealphabetically,
    #[serde(rename = "alphabetically")]
    Alphabetically,
    #[serde(rename = "reversesize")]
    Reversesize,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "reversegitsize")]
    Reversegitsize,
    #[serde(rename = "gitsize")]
    Gitsize,
    #[serde(rename = "reverselfssize")]
    Reverselfssize,
    #[serde(rename = "lfssize")]
    Lfssize,
    #[serde(rename = "moststars")]
    Moststars,
    #[serde(rename = "feweststars")]
    Feweststars,
    #[serde(rename = "mostforks")]
    Mostforks,
    #[serde(rename = "fewestforks")]
    Fewestforks,
}

impl UserCurrentListReposQueryOrderBy {
    fn as_str(&self) -> &'static str {
        match self {
            UserCurrentListReposQueryOrderBy::Name => "name",
            UserCurrentListReposQueryOrderBy::Id => "id",
            UserCurrentListReposQueryOrderBy::Newest => "newest",
            UserCurrentListReposQueryOrderBy::Oldest => "oldest",
            UserCurrentListReposQueryOrderBy::Recentupdate => "recentupdate",
            UserCurrentListReposQueryOrderBy::Leastupdate => "leastupdate",
            UserCurrentListReposQueryOrderBy::Reversealphabetically => "reversealphabetically",
            UserCurrentListReposQueryOrderBy::Alphabetically => "alphabetically",
            UserCurrentListReposQueryOrderBy::Reversesize => "reversesize",
            UserCurrentListReposQueryOrderBy::Size => "size",
            UserCurrentListReposQueryOrderBy::Reversegitsize => "reversegitsize",
            UserCurrentListReposQueryOrderBy::Gitsize => "gitsize",
            UserCurrentListReposQueryOrderBy::Reverselfssize => "reverselfssize",
            UserCurrentListReposQueryOrderBy::Lfssize => "lfssize",
            UserCurrentListReposQueryOrderBy::Moststars => "moststars",
            UserCurrentListReposQueryOrderBy::Feweststars => "feweststars",
            UserCurrentListReposQueryOrderBy::Mostforks => "mostforks",
            UserCurrentListReposQueryOrderBy::Fewestforks => "fewestforks",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserCurrentTrackedTimesQuery {
    /// Only show times updated after the given time. This is a timestamp in RFC 3339 format
    pub since: Option<time::OffsetDateTime>,
    /// Only show times updated before the given time. This is a timestamp in RFC 3339 format
    pub before: Option<time::OffsetDateTime>,
}

impl UserCurrentTrackedTimesQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(since) = self.since {
            list.push((
                "since",
                since
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }
        if let Some(before) = self.before {
            list.push((
                "before",
                before
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserSearchQuery {
    /// keyword
    pub q: Option<String>,
    /// ID of the user to search for
    pub uid: Option<i64>,
    /// sort order of results
    pub sort: Option<UserSearchQuerySort>,
}

impl UserSearchQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(q) = self.q {
            list.push(("q", q));
        }
        if let Some(uid) = self.uid {
            list.push(("uid", uid.to_string()));
        }
        if let Some(sort) = self.sort {
            list.push(("sort", sort.as_str().to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UserSearchQuerySort {
    #[serde(rename = "oldest")]
    Oldest,
    #[serde(rename = "newest")]
    Newest,
    #[serde(rename = "alphabetically")]
    Alphabetically,
    #[serde(rename = "reversealphabetically")]
    Reversealphabetically,
    #[serde(rename = "recentupdate")]
    Recentupdate,
    #[serde(rename = "leastupdate")]
    Leastupdate,
}

impl UserSearchQuerySort {
    fn as_str(&self) -> &'static str {
        match self {
            UserSearchQuerySort::Oldest => "oldest",
            UserSearchQuerySort::Newest => "newest",
            UserSearchQuerySort::Alphabetically => "alphabetically",
            UserSearchQuerySort::Reversealphabetically => "reversealphabetically",
            UserSearchQuerySort::Recentupdate => "recentupdate",
            UserSearchQuerySort::Leastupdate => "leastupdate",
        }
    }
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserSearchResults {
    pub data: Option<Vec<User>>,
    pub ok: Option<bool>,
}

impl_from_response!(UserSearchResults);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserListActivityFeedsQuery {
    /// if true, only show actions performed by the requested user
    pub only_performed_by: Option<bool>,
    /// the date of the activities to be found
    pub date: Option<time::Date>,
}

impl UserListActivityFeedsQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(only_performed_by) = self.only_performed_by {
            list.push(("only-performed-by", only_performed_by.to_string()));
        }
        if let Some(date) = self.date {
            list.push(("date", date.to_string()));
        }

        list
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserListKeysQuery {
    /// fingerprint of the key
    pub fingerprint: Option<String>,
}

impl UserListKeysQuery {
    pub(crate) fn into_list(self) -> Vec<(&'static str, String)> {
        let mut list = Vec::new();
        if let Some(fingerprint) = self.fingerprint {
            list.push(("fingerprint", fingerprint));
        }

        list
    }
}
