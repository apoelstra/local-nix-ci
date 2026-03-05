pub mod endpoints;
pub mod methods;
pub mod structs;

/// lol
pub mod url {
    pub use std::string::ParseError;

    pub struct Url(String);
    impl Url {
        pub fn to_str(&self) -> String { self.0.clone() }
        pub fn parse(s: &str) -> Self { Self(s.clone()) }
    }
}
pub mod reqwest {
    pub mod header {
        pub type HeaderMap = std::collections::HashMap<String, String>;
    }

    pub type StatusCode = u16;

    pub enum Method {
        DELETE,
        GET,
        PATCH,
        POST,
    }
}
#[macro_export]
macro_rules! impl_from_response { ($($anything:tt)*) => {}; }

use core::marker::PhantomData;
use reqwest::StatusCode;
use serde::de::Deserializer;
use std::borrow::Cow;
use url::Url;

// The rest of this file ad-hoc copy/pasted from the real crate
mod sealed {
    pub trait Sealed {}
}

pub trait CountHeader: sealed::Sealed {
    fn count(&self) -> Option<usize>;
}

pub trait PageSize: sealed::Sealed {
    fn page_size(&self) -> usize;
}


pub trait Endpoint: sealed::Sealed {
    type Response;
    fn make_request(self) -> RawRequest;
    fn handle_error(response: ApiResponse) -> Result<(ApiResponse, bool), ForgejoError>;
}

pub struct ApiResponse {
    status_code: StatusCode,
    headers: reqwest::header::HeaderMap,
    body: bytes::Bytes,
}

#[derive(Clone)]
pub struct RawRequest {
    method: reqwest::Method,
    path: Cow<'static, str>,
    query: Option<Vec<(&'static str, String)>>,
    body: RequestBody,
    page: Option<u32>,
    limit: Option<u32>,
}

#[derive(Clone)]
pub enum RequestBody {
    Json(bytes::Bytes),
    Form(Vec<(&'static str, Vec<u8>)>),
    None,
}

pub struct TypedRequest<E, R> {
    inner: RawRequest,
    __endpoint: PhantomData<E>,
    __response: PhantomData<R>,
}



#[derive(thiserror::Error, Debug)]
pub enum ForgejoError {
    #[error("url must have a host")]
    HostRequired,
    #[error("scheme must be http or https")]
    HttpRequired,
    /*
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    */
    #[error("API key should be ascii")]
    KeyNotAscii,
    #[error("the response from forgejo was not properly structured")]
    BadStructure(#[from] StructureError),
    #[error("unexpected status code {} {}", .0.as_u16(), .0.canonical_reason().unwrap_or(""))]
    UnexpectedStatusCode(StatusCode),
    #[error(transparent)]
    ApiError(#[from] ApiError),
    #[error("the provided authorization was too long to accept")]
    AuthTooLong,
}

#[derive(thiserror::Error, Debug)]
pub enum StructureError {
    #[error("{e}")]
    Serde {
        e: serde_json::Error,
        contents: bytes::Bytes,
    },
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error("failed to find header `{0}`")]
    HeaderMissing(&'static str),
    #[error("header was not ascii")]
    HeaderNotAscii,
    #[error("failed to parse header")]
    HeaderParseFailed,
    #[error("nothing was returned when a value was expected")]
    EmptyResponse,
}

impl From<std::str::Utf8Error> for ForgejoError {
    fn from(error: std::str::Utf8Error) -> Self {
        Self::BadStructure(StructureError::Utf8(error))
    }
}

#[derive(thiserror::Error, Debug)]
pub struct ApiError {
    pub message: Option<String>,
    pub kind: ApiErrorKind,
}

impl ApiError {
    fn new(message: Option<String>, kind: ApiErrorKind) -> Self {
        Self { message, kind }
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn error_kind(&self) -> &ApiErrorKind {
        &self.kind
    }
}

impl From<ApiErrorKind> for ApiError {
    fn from(kind: ApiErrorKind) -> Self {
        Self {
            message: None,
            kind,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.message {
            Some(message) => write!(f, "{}: {message}", self.kind),
            None => write!(f, "{}", self.kind),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ApiErrorKind {
    #[error("api error")]
    Generic,
    #[error("access denied")]
    Forbidden,
    #[error("invalid topics")]
    InvalidTopics { invalid_topics: Option<Vec<String>> },
    #[error("not found")]
    NotFound { errors: Option<Vec<String>> },
    #[error("repo archived")]
    RepoArchived,
    #[error("unauthorized")]
    Unauthorized,
    #[error("validation failed")]
    ValidationFailed,
    #[error("status code {0}")]
    Other(reqwest::StatusCode),
}

impl From<structs::APIError> for ApiError {
    fn from(value: structs::APIError) -> Self {
        Self::new(value.message, ApiErrorKind::Generic)
    }
}
impl From<structs::APIForbiddenError> for ApiError {
    fn from(value: structs::APIForbiddenError) -> Self {
        Self::new(value.message, ApiErrorKind::Forbidden)
    }
}
impl From<structs::APIInvalidTopicsError> for ApiError {
    fn from(value: structs::APIInvalidTopicsError) -> Self {
        Self::new(
            value.message,
            ApiErrorKind::InvalidTopics {
                invalid_topics: value.invalid_topics,
            },
        )
    }
}
impl From<structs::APINotFound> for ApiError {
    fn from(value: structs::APINotFound) -> Self {
        Self::new(
            value.message,
            ApiErrorKind::NotFound {
                errors: value.errors,
            },
        )
    }
}
impl From<structs::APIRepoArchivedError> for ApiError {
    fn from(value: structs::APIRepoArchivedError) -> Self {
        Self::new(value.message, ApiErrorKind::RepoArchived)
    }
}
impl From<structs::APIUnauthorizedError> for ApiError {
    fn from(value: structs::APIUnauthorizedError) -> Self {
        Self::new(value.message, ApiErrorKind::Unauthorized)
    }
}
impl From<structs::APIValidationError> for ApiError {
    fn from(value: structs::APIValidationError) -> Self {
        Self::new(value.message, ApiErrorKind::ValidationFailed)
    }
}
impl From<reqwest::StatusCode> for ApiError {
    fn from(value: reqwest::StatusCode) -> Self {
        match value {
            reqwest::StatusCode::NOT_FOUND => ApiErrorKind::NotFound { errors: None },
            reqwest::StatusCode::FORBIDDEN => ApiErrorKind::Forbidden,
            reqwest::StatusCode::UNAUTHORIZED => ApiErrorKind::Unauthorized,
            _ => ApiErrorKind::Other(value),
        }
        .into()
    }
}

// Forgejo can return blank strings for URLs. This handles that by deserializing
// that as `None`
fn none_if_blank_url<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Url>, D::Error> {
    use serde::de::{Error, Unexpected, Visitor};
    use std::fmt;

    struct EmptyUrlVisitor;

    impl<'de> Visitor<'de> for EmptyUrlVisitor {
        type Value = Option<Url>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("option")
        }

        #[inline]
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        #[inline]
        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        #[inline]
        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s: String = serde::Deserialize::deserialize(deserializer)?;
            if s.is_empty() {
                return Ok(None);
            }
            Url::parse(&s)
                .map_err(|err| {
                    let err_s = format!("{}", err);
                    Error::invalid_value(Unexpected::Str(&s), &err_s.as_str())
                })
                .map(Some)
        }

        #[inline]
        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            if s.is_empty() {
                return Ok(None);
            }
            Url::parse(s)
                .map_err(|err| {
                    let err_s = format!("{err}");
                    Error::invalid_value(Unexpected::Str(s), &err_s.as_str())
                })
                .map(Some)
        }
    }

    deserializer.deserialize_option(EmptyUrlVisitor)
}

#[allow(dead_code)] // not used yet, but it might appear in the future
fn deserialize_ssh_url<'de, D, DE>(deserializer: D) -> Result<Url, DE>
where
    D: Deserializer<'de>,
    DE: serde::de::Error,
{
    let raw_url: String = String::deserialize(deserializer).map_err(DE::custom)?;
    parse_ssh_url(&raw_url).map_err(DE::custom)
}

fn deserialize_optional_ssh_url<'de, D, DE>(deserializer: D) -> Result<Option<Url>, DE>
where
    D: Deserializer<'de>,
    DE: serde::de::Error,
{
    let raw_url: Option<String> = Option::deserialize(deserializer).map_err(DE::custom)?;
    raw_url
        .as_ref()
        .map(parse_ssh_url)
        .map(|res| res.map_err(DE::custom))
        .transpose()
        .or(Ok(None))
}

fn requested_reviewers_ignore_null<'de, D, DE>(
    deserializer: D,
) -> Result<Option<Vec<structs::User>>, DE>
where
    D: Deserializer<'de>,
    DE: serde::de::Error,
{
    let list: Option<Vec<Option<structs::User>>> =
        Option::deserialize(deserializer).map_err(DE::custom)?;
    Ok(list.map(|list| list.into_iter().flatten().collect::<Vec<_>>()))
}

fn parse_ssh_url(raw_url: &String) -> Result<Url, url::ParseError> {
    // in case of a non-standard ssh-port (not 22), the ssh url coming from the forgejo API
    // is actually parseable by the url crate, so try to do that first
    Url::parse(raw_url).or_else(|_| {
        // otherwise the ssh url is not parseable by the url crate and we try again after some
        // pre-processing
        let url = format!("ssh://{url}", url = raw_url.replace(":", "/"));
        Url::parse(url.as_str())
    })
}


