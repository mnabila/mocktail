//! Request-log use case + repository port.

use super::domain::{RequestId, RequestLog};
use crate::shared::error::AppResult;
use crate::shared::ids::{CollectionId, WorkspaceId};
use crate::shared::ports::UnixMillis;
use async_trait::async_trait;

/// Keyset cursor into the log stream: the (received_at, id) of the last row a
/// page returned. The next page continues strictly before it.
#[derive(Clone, Debug)]
pub struct LogCursor {
    pub received_at: UnixMillis,
    pub id: String,
}

/// Filters + keyset pagination for the log viewer.
#[derive(Default)]
pub struct LogQuery<'a> {
    pub coll: Option<&'a CollectionId>,
    /// Case-sensitive substring match against the request path.
    pub path_contains: Option<&'a str>,
    /// Exact status code match.
    pub status: Option<u16>,
    /// Status-class lower bound (200/300/400/500); matches `[class, class+100)`.
    /// Ignored when `status` is set.
    pub status_class: Option<u16>,
    pub cursor: Option<LogCursor>,
    pub limit: usize,
}

#[async_trait]
pub trait RequestLogRepository: Send + Sync {
    async fn append(&self, log: &RequestLog) -> AppResult<()>;
    async fn find_by_request_id(
        &self,
        ws: &WorkspaceId,
        request_id: &RequestId,
    ) -> AppResult<Option<RequestLog>>;
    /// Most recent logs for a collection, newest first (portal viewer).
    async fn recent(
        &self,
        ws: &WorkspaceId,
        coll: Option<&CollectionId>,
        limit: usize,
    ) -> AppResult<Vec<RequestLog>>;
    /// Filtered, keyset-paginated logs, newest first. Returns up to `q.limit`
    /// rows; the caller derives the next cursor from the last row.
    async fn page(&self, ws: &WorkspaceId, q: &LogQuery<'_>) -> AppResult<Vec<RequestLog>>;
}

/// Append a request log entry. Best-effort at the call site:
/// a logging failure must never fail the mock response.
pub async fn record_request(repo: &dyn RequestLogRepository, log: &RequestLog) -> AppResult<()> {
    repo.append(log).await
}
