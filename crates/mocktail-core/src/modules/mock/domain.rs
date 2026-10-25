//! Mock (stub) entity and its matcher/response value objects.
//! Pure data + invariants; matching logic lives in the matching feature.

use crate::shared::http::Method;
use crate::shared::ids::{CollectionId, FolderId, MockId, WorkspaceId};
use crate::shared::ports::UnixMillis;

/// What part of the request a matcher inspects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchTarget {
    Query(String),
    Header(String),
    /// Whole request body (as UTF-8 text).
    Body,
}

/// How a matcher compares the target value.
#[derive(Clone, Debug)]
pub enum MatchKind {
    Exact(String),
    Contains(String),
    Regex(String),
    /// JSONPath-ish lookup into a JSON body: a `/`-separated pointer and an
    /// optional expected value. See matching engine for evaluation.
    JsonPath {
        path: String,
        equals: Option<String>,
    },
}

impl PartialEq for MatchKind {
    fn eq(&self, other: &Self) -> bool {
        use MatchKind::*;
        match (self, other) {
            (Exact(a), Exact(b)) => a == b,
            (Contains(a), Contains(b)) => a == b,
            (Regex(a), Regex(b)) => a == b,
            (
                JsonPath {
                    path: p1,
                    equals: e1,
                },
                JsonPath {
                    path: p2,
                    equals: e2,
                },
            ) => p1 == p2 && e1 == e2,
            _ => false,
        }
    }
}
impl Eq for MatchKind {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestMatcher {
    pub target: MatchTarget,
    pub kind: MatchKind,
}

/// A simulated fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// Drop the connection with no response.
    ConnectionReset,
    /// Return malformed data instead of the configured body.
    MalformedResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseSpec {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    /// Response body; may contain `{{param}}` templating placeholders filled
    /// from captured path params.
    pub body: Vec<u8>,
}

/// Explicit integer priority; higher wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Priority(pub i32);

/// Per-endpoint request throttle. Only the client IP is used as the counter
/// key; the fixed window is implementation-defined (per process/isolate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateLimit {
    pub limit: u32,
    pub window_secs: u32,
    /// HTTP status returned when the limit is exceeded (usually 429).
    pub status: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mock {
    pub id: MockId,
    pub workspace_id: WorkspaceId,
    pub collection_id: CollectionId,
    /// Optional grouping folder inside the collection.
    pub folder_id: Option<FolderId>,
    pub method: Method,
    /// Spec-relative path template, e.g. `/users/{id}`.
    pub path_template: String,
    pub matchers: Vec<RequestMatcher>,
    pub response: ResponseSpec,
    pub priority: Priority,
    pub delay_ms: u32,
    pub fault: Option<Fault>,
    pub rate_limit: Option<RateLimit>,
    pub created_at: UnixMillis,
    pub updated_at: UnixMillis,
}
