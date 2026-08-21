// SPDX-License-Identifier: GPL-3.0-or-later

pub mod serde_types;

use core::fmt;

use chrono::{DateTime, Utc};
use xshell::cmd;

use self::serde_types::{Comment, CommitEntry, PullRequest, Review};
use crate::git::CommitId;
use crate::{gh, repo, PrNumber};

const USER_AGENT: &str = "curl/8.5.0";
const LIST_PAGE_SIZE: usize = 50;

#[derive(Debug)]
pub enum LoadError {
    GitConfigMissing(&'static str),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitConfigMissing(k) => write!(f, "missing git config value `{}`", k),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::GitConfigMissing(..) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgejoRepoData {
    token: String,
    username: String,
    https_url: String,
}

fn read_git_config(sh: &xshell::Shell, key: &'static str) -> Option<String> {
    match cmd!(sh, "git config --get {key}").read() {
        Ok(v) => Some(v.trim().to_string()),
        // xshell returns an error on nonzero exit; treat that as "unset".
        Err(_) => None,
    }
}

impl ForgejoRepoData {
    /// Loads Forgejo data from a shell already cd'd into the correct directory.
    ///
    /// The base HTTPS URL is derived from the origin remote by way of [`repo::detect_upstream`],
    /// unless `forgejo.httpsUrl` is set in git config, in which case that value takes precedence.
    pub(crate) fn load_from_shell(
        shell: &xshell::Shell,
        default_https_url: String,
    ) -> Result<Self, LoadError> {
        let token = read_git_config(shell, "forgejo.token")
            .ok_or(LoadError::GitConfigMissing("forgejo.token"))?;
        let username = read_git_config(shell, "forgejo.username")
            .ok_or(LoadError::GitConfigMissing("forgejo.username"))?;
        let override_https_url = read_git_config(shell, "forgejo.httpsUrl");

        let https_url = override_https_url.unwrap_or(default_https_url);
        Ok(Self {
            token,
            username,
            https_url,
        })
    }

    async fn api_get<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &impl fmt::Display,
    ) -> Result<T, ApiError> {
        let url = format!("{}/api/v1/{}", self.https_url, endpoint);
        let response = bitreq::get(&url)
            .with_header("Authorization", format!("token {}", self.token))
            .with_header("Accept", "*/*")
            .with_header("User-Agent", USER_AGENT)
            .send_async()
            .await
            .map_err(|e| ApiError::Http(url.clone(), e))?;
        if response.status_code < 200 || response.status_code >= 300 {
            let body = response.as_str().unwrap_or("").to_string();
            return Err(ApiError::HttpStatus {
                url,
                status: response.status_code,
                body,
            });
        }
        let body = response
            .as_str()
            .map_err(|e| ApiError::Http(url.clone(), e))?
            .to_string();
        serde_json::from_str(&body).map_err(|e| ApiError::Json(body, e))
    }

    async fn api_post<B: serde::Serialize>(
        &self,
        endpoint: &impl fmt::Display,
        body: &B,
    ) -> Result<(), ApiError> {
        let body_json = serde_json::to_string(body).map_err(ApiError::JsonSerialize)?;
        let url = format!("{}/api/v1/{}", self.https_url, endpoint);
        let response = bitreq::post(&url)
            .with_header("Authorization", format!("token {}", self.token))
            .with_header("Accept", "*/*")
            .with_header("Content-Type", "application/json")
            .with_header("User-Agent", USER_AGENT)
            .with_body(body_json.as_str())
            .send_async()
            .await
            .map_err(|e| ApiError::Http(url.clone(), e))?;
        if response.status_code < 200 || response.status_code >= 300 {
            let body = response.as_str().unwrap_or("").to_string();
            return Err(ApiError::HttpStatus {
                url,
                status: response.status_code,
                body,
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ApiError {
    JsonSerialize(serde_json::Error),
    Http(String, bitreq::Error),
    HttpStatus {
        url: String,
        status: i32,
        body: String,
    },
    Json(String, serde_json::Error),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JsonSerialize(_) => f.write_str("failed to serialize JSON body"),
            Self::Http(url, _) => write!(f, "HTTP request failed: {}", url),
            Self::HttpStatus { url, status, body } => {
                write!(f, "HTTP {} from {}: {}", status, url, body)
            }
            Self::Json(json, _) => write!(f, "failed to parse JSON response: {}", json),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::JsonSerialize(e) => Some(e),
            Self::Http(_, e) => Some(e),
            Self::Json(_, e) => Some(e),
            Self::HttpStatus { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Shell(String, xshell::Error),
    ShellLock(tokio::task::JoinError),
    Repo(repo::RepoError),
    NotForgejoRemote,
    PrNotFound(PrNumber),
    ApiGet(String, ApiError),
    ApiPost(String, ApiError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shell(cmd, _) => write!(f, "failed to invoke command: {}", cmd),
            Self::ShellLock(_) => f.write_str("panic while holding shell lock"),
            Self::Repo(_) => f.write_str("failed to detect repository upstream"),
            Self::NotForgejoRemote => {
                f.write_str("origin remote is not a recognized Forgejo remote")
            }
            Self::PrNotFound(n) => write!(f, "PR #{} not found", n),
            Self::ApiGet(endpoint, _) => write!(f, "failed API GET request to {endpoint}"),
            Self::ApiPost(endpoint, _) => write!(f, "failed API POST request to {endpoint}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Shell(_, e) => Some(e),
            Self::ShellLock(e) => Some(e),
            Self::Repo(e) => Some(e),
            Self::ApiGet(_, e) => Some(e),
            Self::ApiPost(_, e) => Some(e),
            Self::NotForgejoRemote => None,
            Self::PrNotFound(..) => None,
        }
    }
}

/// Fetches PR information from Forgejo. Mirrors [`gh::get_pr_info`].
///
/// # Errors
///
/// Returns an error on shell failure, missing git config, unrecognized origin remote, network
/// failure, unexpected HTTP status, or JSON parse failure. Returns `Error::PrNotFound` when the
/// server returns 404.
pub(crate) async fn get_pr_info(
    project_name: &str,
    repo_data: &ForgejoRepoData,
    pr_number: PrNumber,
) -> Result<gh::PrInfo, Error> {
    let pr_endpoint = format!("repos/{}/pulls/{}", project_name, pr_number);
    let pr: PullRequest = match repo_data.api_get(&pr_endpoint).await {
        Ok(v) => v,
        Err(ApiError::HttpStatus { status: 404, .. }) => return Err(Error::PrNotFound(pr_number)),
        Err(e) => return Err(Error::ApiGet(pr_endpoint, e)),
    };

    let commits_endpoint = format!("repos/{}/pulls/{}/commits", project_name, pr_number);
    let commits: Vec<CommitEntry> = repo_data.api_get(&commits_endpoint).await
        .map_err(|e| Error::ApiGet(commits_endpoint, e))?;
    // Note that Forgejo shows the commits in the opposite order of Github so we have to reverse.
    let commits: Vec<CommitId> = commits.into_iter().rev().map(|c| c.sha).collect();

    let comments_endpoint = format!("repos/{}/issues/{}/comments", project_name, pr_number);
    let comments: Vec<Comment> = repo_data.api_get(&comments_endpoint).await
        .map_err(|e| Error::ApiGet(comments_endpoint, e))?;

    let reviews_endpoint = format!("repos/{}/pulls/{}/reviews", project_name, pr_number);
    let reviews: Vec<Review> = repo_data.api_get(&reviews_endpoint).await
        .map_err(|e| Error::ApiGet(reviews_endpoint, e))?;

    Ok(pr.into_gh_pr_info(commits, comments, reviews))
}

/// Lists PRs updated since the given timestamp. Mirrors [`gh::list_updated_prs`].
///
/// Fetches pages of open PRs sorted by most-recently-updated and filters client-side.
///
/// # Errors
///
/// Returns an error on shell failure, missing git config, unrecognized origin remote, network
/// failure, unexpected HTTP status, or JSON parse failure.
pub(crate) async fn list_updated_prs(
    project_name: &str,
    repo_data: &ForgejoRepoData,
    since: DateTime<Utc>,
) -> Result<Vec<gh::PrInfo>, Error> {
    let mut out = Vec::new();
    let mut page = 1;
    let mut saved_err: Option<Error> = None;
    loop {
        // If you are a masochist, change this format! to format_args! and try to parse the
        // resulting async-related compiler error.
        let endpoint = format!(
            "repos/{}/pulls?state=open&sort=recentupdate&limit={}&page={}",
            project_name, LIST_PAGE_SIZE, page
        );
        let prs: Vec<PullRequest> = repo_data.api_get(&endpoint).await
            .map_err(|e| Error::ApiPost(endpoint, e))?;
        if prs.is_empty() {
            break;
        }
        let mut hit_old = false;
        for pr in prs {
            if pr.updated_at < since {
                hit_old = true;
                continue;
            }
            match get_pr_info(project_name, repo_data, pr.number).await {
                Ok(info) => out.push(info),
                Err(e) => saved_err = Some(e),
            }
        }
        if hit_old {
            break;
        }
        page += 1;
    }
    if let Some(e) = saved_err {
        return Err(e);
    }
    Ok(out)
}

/// Posts a comment on a Forgejo PR. Mirrors [`gh::post_pr_comment`].
///
/// # Errors
///
/// Returns an error on shell failure, missing git config, unrecognized origin remote, network
/// failure, unexpected HTTP status, or JSON serialization failure.
pub(crate) async fn post_pr_comment(
    project_name: &str,
    repo_data: &ForgejoRepoData,
    pr_number: PrNumber,
    comment: &str,
) -> Result<(), Error> {
    #[derive(serde::Serialize)]
    struct Body<'a> {
        body: &'a str,
    }

    let endpoint = format!("repos/{}/issues/{}/comments", project_name, pr_number);
    repo_data.api_post(&endpoint, &Body { body: comment }).await
        .map_err(|e| Error::ApiPost(endpoint, e))
}

/// Posts an approving review on a Forgejo PR. Mirrors [`gh::post_pr_approval`], but requires
/// the reviewed commit id.
///
/// # Errors
///
/// Returns an error on shell failure, missing git config, unrecognized origin remote, network
/// failure, unexpected HTTP status, or JSON serialization failure.
pub(crate) async fn post_pr_approval(
    project_name: &str,
    repo_data: &ForgejoRepoData,
    pr_number: PrNumber,
    commit_id: &CommitId,
    message: &str,
) -> Result<(), Error> {
    #[derive(serde::Serialize)]
    struct Body<'a> {
        body: &'a str,
        commit_id: String,
        event: &'a str,
    }

    let endpoint = format!("repos/{}/pulls/{}/reviews", project_name, pr_number);
    repo_data
        .api_post(
            &endpoint,
            &Body {
                body: message,
                commit_id: commit_id.to_string(),
                event: "APPROVED",
            },
        )
        .await
        .map_err(|e| Error::ApiPost(endpoint, e))
}
